use serde::Serialize;
#[cfg(windows)]
use std::path::Path;
#[cfg(any(target_os = "macos", target_os = "linux"))]
use std::process::Command;

use crate::settings::{BackendSettings, relay_profile_resolved_api_key};

const DEFAULT_CODEX_AUTH_ENV_KEY: &str = "OPENAI_API_KEY";
#[cfg(windows)]
const WINDOWS_USER_ENVIRONMENT_KEY: &str = "Environment";
#[cfg(windows)]
const WINDOWS_SYSTEM_ENVIRONMENT_KEY: &str =
    r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CredentialEnvironmentDiagnostic {
    pub variable_name: String,
    pub present: bool,
    pub conflict: bool,
    pub process_present: bool,
    pub user_present: bool,
    pub system_present: bool,
    pub can_clear_user: bool,
    pub profile_has_key: bool,
    pub restart_required: bool,
    pub user_scope: String,
    pub user_scope_available: bool,
    pub user_scope_error: Option<String>,
    pub external_source_likely: bool,
}

#[cfg(windows)]
struct TemporaryUserCredentialEnvironment {
    variable_name: String,
    previous_user_value: Option<crate::windows_integration::RegistryStringValue>,
    previous_process_value: Option<std::ffi::OsString>,
    restored: bool,
}

#[cfg(windows)]
struct TemporaryCodexCredentialEnvironmentLock {
    _guard: crate::windows_integration::NamedMutexGuard,
}

#[cfg(windows)]
impl TemporaryCodexCredentialEnvironmentLock {
    fn acquire() -> anyhow::Result<Self> {
        Ok(Self {
            _guard: crate::windows_integration::acquire_named_mutex(
                r"Local\ClaudeCodexPro.CodexCredentialEnvironment.v1",
            )?,
        })
    }
}

#[cfg(windows)]
impl TemporaryUserCredentialEnvironment {
    fn apply(variable_name: &str, credential: Option<&str>) -> anyhow::Result<Self> {
        if !valid_environment_variable_name(variable_name) {
            anyhow::bail!("Codex provider env_key is invalid");
        }
        let credential = credential.map(str::trim);
        if credential.is_some_and(str::is_empty) {
            anyhow::bail!("Codex provider credential is empty");
        }

        let previous_user_value =
            crate::windows_integration::current_user_registry_string_value_result(
                WINDOWS_USER_ENVIRONMENT_KEY,
                variable_name,
            )?;
        let previous_process_value = std::env::var_os(variable_name);
        match credential {
            Some(credential) => {
                if previous_user_value
                    .as_ref()
                    .map(|value| value.value.as_str())
                    != Some(credential)
                {
                    crate::windows_integration::set_current_user_string_value(
                        WINDOWS_USER_ENVIRONMENT_KEY,
                        variable_name,
                        credential,
                    )?;
                }
                if previous_process_value.as_deref() != Some(std::ffi::OsStr::new(credential)) {
                    unsafe {
                        std::env::set_var(variable_name, credential);
                    }
                }
            }
            None => {
                if previous_user_value.is_some() {
                    crate::windows_integration::delete_current_user_value(
                        WINDOWS_USER_ENVIRONMENT_KEY,
                        variable_name,
                    )?;
                }
                if previous_process_value.is_some() {
                    unsafe {
                        std::env::remove_var(variable_name);
                    }
                }
            }
        }

        Ok(Self {
            variable_name: variable_name.to_string(),
            previous_user_value,
            previous_process_value,
            restored: false,
        })
    }

    fn restore(&mut self) -> anyhow::Result<()> {
        let registry_result = match self.previous_user_value.as_ref() {
            Some(value) => crate::windows_integration::set_current_user_registry_string_value(
                WINDOWS_USER_ENVIRONMENT_KEY,
                &self.variable_name,
                value,
            ),
            None => crate::windows_integration::delete_current_user_value(
                WINDOWS_USER_ENVIRONMENT_KEY,
                &self.variable_name,
            ),
        }
        .and_then(|()| {
            let restored = crate::windows_integration::current_user_registry_string_value_result(
                WINDOWS_USER_ENVIRONMENT_KEY,
                &self.variable_name,
            )?;
            if restored == self.previous_user_value {
                Ok(())
            } else {
                anyhow::bail!("temporary Codex credential environment was not restored")
            }
        });

        match self.previous_process_value.as_ref() {
            Some(value) => unsafe { std::env::set_var(&self.variable_name, value) },
            None => unsafe { std::env::remove_var(&self.variable_name) },
        }
        if registry_result.is_ok() {
            self.restored = true;
        }
        registry_result
    }
}

#[cfg(windows)]
impl Drop for TemporaryUserCredentialEnvironment {
    fn drop(&mut self) {
        if !self.restored {
            let _ = self.restore();
        }
    }
}

/// Give a newly activated Windows MSIX Codex process the live provider
/// credential without leaving it in the user's persistent environment.
#[cfg(windows)]
pub fn with_temporary_codex_user_credential_environment_from_home<T>(
    home: &Path,
    operation: impl FnOnce() -> anyhow::Result<T>,
) -> anyhow::Result<T> {
    let provider_environment = crate::relay_config::codex_provider_auth_environment_from_home(home);
    let mut variable_names =
        crate::relay_config::codex_provider_credential_environment_keys_from_home(home);
    if let Some((variable_name, _)) = provider_environment.as_ref()
        && !variable_names.contains(variable_name)
    {
        variable_names.push(variable_name.clone());
    }
    // The registry and process environment are shared mutable launch state;
    // serialize the save/activate/restore window across launcher processes.
    let _environment_lock = TemporaryCodexCredentialEnvironmentLock::acquire()?;
    let mut environments = Vec::with_capacity(variable_names.len());
    for variable_name in variable_names {
        let credential = provider_environment
            .as_ref()
            .filter(|(active_name, _)| active_name == &variable_name)
            .map(|(_, credential)| credential.as_str());
        environments.push(TemporaryUserCredentialEnvironment::apply(
            &variable_name,
            credential,
        )?);
    }
    let operation_result = operation();
    let mut restore_result = Ok(());
    for environment in environments.iter_mut().rev() {
        if let Err(error) = environment.restore()
            && restore_result.is_ok()
        {
            restore_result = Err(error);
        }
    }
    match (operation_result, restore_result) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(restore_error)) => Err(restore_error),
        (Err(operation_error), Err(restore_error)) => Err(anyhow::anyhow!(
            "Codex activation failed: {operation_error:#}; restoring temporary credential environment also failed: {restore_error:#}"
        )),
    }
}

pub fn current_user_credential_environment_value_result(
    variable_name: &str,
) -> anyhow::Result<Option<String>> {
    if !valid_environment_variable_name(variable_name) {
        anyhow::bail!("Codex provider env_key is invalid");
    }

    #[cfg(windows)]
    {
        return crate::windows_integration::current_user_registry_string_value_result(
            WINDOWS_USER_ENVIRONMENT_KEY,
            variable_name,
        )
        .map(|value| value.map(|value| value.value));
    }

    #[cfg(target_os = "macos")]
    {
        let output = Command::new("launchctl")
            .args(["getenv", variable_name])
            .output()
            .map_err(|error| anyhow::anyhow!("读取 launchd 用户会话环境失败：{error}"))?;
        if !output.status.success() {
            if output.status.code() == Some(1) {
                return Ok(None);
            }
            anyhow::bail!("读取 launchd 用户会话环境失败")
        }
        return Ok(non_empty(
            String::from_utf8_lossy(&output.stdout)
                .trim()
                .to_string()
                .into(),
        ));
    }

    #[cfg(target_os = "linux")]
    {
        let output = Command::new("systemctl")
            .args(["--user", "show-environment"])
            .output()
            .map_err(|error| anyhow::anyhow!("读取 systemd 用户环境失败：{error}"))?;
        if !output.status.success() {
            anyhow::bail!("读取 systemd 用户环境失败")
        }
        let prefix = format!("{variable_name}=");
        return Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .find_map(|line| line.strip_prefix(&prefix))
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string));
    }

    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    Ok(None)
}

fn clear_current_user_credential_environment_value(variable_name: &str) -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        return crate::windows_integration::delete_current_user_value(
            WINDOWS_USER_ENVIRONMENT_KEY,
            variable_name,
        );
    }

    #[cfg(target_os = "macos")]
    {
        let status = Command::new("launchctl")
            .args(["unsetenv", variable_name])
            .status()
            .map_err(|error| anyhow::anyhow!("清理 launchd 用户会话环境失败：{error}"))?;
        if !status.success() {
            anyhow::bail!("清理 launchd 用户会话环境失败")
        }
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        let status = Command::new("systemctl")
            .args(["--user", "unset-environment", variable_name])
            .status()
            .map_err(|error| anyhow::anyhow!("清理 systemd 用户环境失败：{error}"))?;
        if !status.success() {
            anyhow::bail!("清理 systemd 用户环境失败")
        }
        return Ok(());
    }

    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    anyhow::bail!("当前平台不支持由 CCP 清理用户会话环境变量")
}

pub fn diagnose_codex_credential_environment(
    settings: &BackendSettings,
) -> CredentialEnvironmentDiagnostic {
    let profile = settings.active_relay_profile();
    let variable_name = profile_env_key(&profile.config_contents);
    let profile_key = relay_profile_resolved_api_key(&profile);
    let process_value = non_empty(std::env::var(&variable_name).ok());

    let user_value_result = current_user_credential_environment_value_result(&variable_name);
    let user_value = user_value_result
        .as_ref()
        .ok()
        .and_then(|value| value.clone());

    #[cfg(windows)]
    let system_value = crate::windows_integration::local_machine_string_value(
        WINDOWS_SYSTEM_ENVIRONMENT_KEY,
        &variable_name,
    );
    #[cfg(not(windows))]
    let system_value: Option<String> = None;

    let mut diagnostic = analyze_credential_environment(
        &variable_name,
        &profile_key,
        process_value.as_deref(),
        user_value.as_deref(),
        system_value.as_deref(),
    );
    diagnostic.user_scope_available = user_value_result.is_ok();
    diagnostic.user_scope_error = user_value_result.err().map(|error| error.to_string());
    diagnostic.can_clear_user =
        diagnostic.process_present || (diagnostic.user_scope_available && diagnostic.user_present);
    diagnostic
}

pub fn analyze_credential_environment(
    variable_name: &str,
    profile_key: &str,
    process_value: Option<&str>,
    user_value: Option<&str>,
    system_value: Option<&str>,
) -> CredentialEnvironmentDiagnostic {
    let profile_key = profile_key.trim();
    let process_value = non_empty(process_value.map(ToOwned::to_owned));
    let user_value = non_empty(user_value.map(ToOwned::to_owned));
    let system_value = non_empty(system_value.map(ToOwned::to_owned));
    let values = [
        process_value.as_deref(),
        user_value.as_deref(),
        system_value.as_deref(),
    ];
    let present = values.iter().any(Option::is_some);
    let conflict = !profile_key.is_empty()
        && values
            .iter()
            .flatten()
            .any(|value| value.trim() != profile_key);

    CredentialEnvironmentDiagnostic {
        variable_name: variable_name.to_string(),
        present,
        conflict,
        process_present: process_value.is_some(),
        user_present: user_value.is_some(),
        system_present: system_value.is_some(),
        can_clear_user: process_value.is_some() || user_value.is_some(),
        profile_has_key: !profile_key.is_empty(),
        restart_required: false,
        user_scope: current_user_environment_scope().to_string(),
        user_scope_available: true,
        user_scope_error: None,
        external_source_likely: process_value.is_some()
            && user_value.is_none()
            && system_value.is_none(),
    }
}

fn current_user_environment_scope() -> &'static str {
    #[cfg(windows)]
    {
        return "windows-user-environment";
    }
    #[cfg(target_os = "macos")]
    {
        return "macos-launchd-user-session";
    }
    #[cfg(target_os = "linux")]
    {
        return "linux-systemd-user-manager";
    }
    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    "unsupported-user-session"
}

pub fn clear_codex_user_credential_environment(
    settings: &BackendSettings,
    requested_name: &str,
) -> anyhow::Result<CredentialEnvironmentDiagnostic> {
    let profile = settings.active_relay_profile();
    let expected_name = profile_env_key(&profile.config_contents);
    if !valid_environment_variable_name(requested_name) || requested_name != expected_name {
        anyhow::bail!("环境变量名称无效或已不再属于当前 Codex 供应商");
    }
    #[cfg(windows)]
    let _environment_lock = TemporaryCodexCredentialEnvironmentLock::acquire()?;
    let external_source_likely =
        diagnose_codex_credential_environment(settings).external_source_likely;

    clear_current_user_credential_environment_value(requested_name)?;
    unsafe {
        std::env::remove_var(requested_name);
    }
    let mut diagnostic = diagnose_codex_credential_environment(settings);
    diagnostic.restart_required = true;
    diagnostic.external_source_likely |= external_source_likely;
    Ok(diagnostic)
}

pub fn valid_environment_variable_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn profile_env_key(config_contents: &str) -> String {
    let parsed = config_contents.parse::<toml::Value>().ok();
    let provider_id = parsed
        .as_ref()
        .and_then(|value| value.get("model_provider"))
        .and_then(toml::Value::as_str);
    provider_id
        .and_then(|provider_id| {
            parsed
                .as_ref()
                .and_then(|value| value.get("model_providers"))
                .and_then(|providers| providers.get(provider_id))
                .and_then(|provider| provider.get("env_key"))
                .and_then(toml::Value::as_str)
        })
        .map(str::trim)
        .filter(|value| valid_environment_variable_name(value))
        .unwrap_or(DEFAULT_CODEX_AUTH_ENV_KEY)
        .to_string()
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|value| !value.trim().is_empty())
}
