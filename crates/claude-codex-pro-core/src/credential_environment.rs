use serde::Serialize;

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
}

pub fn diagnose_codex_credential_environment(
    settings: &BackendSettings,
) -> CredentialEnvironmentDiagnostic {
    let profile = settings.active_relay_profile();
    let variable_name = profile_env_key(&profile.config_contents);
    let profile_key = relay_profile_resolved_api_key(&profile);
    let process_value = non_empty(std::env::var(&variable_name).ok());

    #[cfg(windows)]
    let user_value = crate::windows_integration::current_user_string_value(
        WINDOWS_USER_ENVIRONMENT_KEY,
        &variable_name,
    );
    #[cfg(not(windows))]
    let user_value: Option<String> = None;

    #[cfg(windows)]
    let system_value = crate::windows_integration::local_machine_string_value(
        WINDOWS_SYSTEM_ENVIRONMENT_KEY,
        &variable_name,
    );
    #[cfg(not(windows))]
    let system_value: Option<String> = None;

    analyze_credential_environment(
        &variable_name,
        &profile_key,
        process_value.as_deref(),
        user_value.as_deref(),
        system_value.as_deref(),
    )
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
        can_clear_user: cfg!(windows) && user_value.is_some(),
        profile_has_key: !profile_key.is_empty(),
        restart_required: false,
    }
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
    {
        crate::windows_integration::delete_current_user_value(
            WINDOWS_USER_ENVIRONMENT_KEY,
            requested_name,
        )?;
        unsafe {
            std::env::remove_var(requested_name);
        }
        let mut diagnostic = diagnose_codex_credential_environment(settings);
        diagnostic.restart_required = true;
        return Ok(diagnostic);
    }

    #[cfg(not(windows))]
    anyhow::bail!("当前平台不支持由 CCP 清理持久化用户环境变量")
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
