use anyhow::{Context, Result, bail};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{BufRead, BufReader};
#[cfg(not(windows))]
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::mpsc;
use uuid::Uuid;

const PACKAGE_VERSION: &str = "1.0.7";
const PROMPT: &str = "gpt5.5-unrestricted.md";
const IDENTITY: &str = "leila-identity";
const AC: &str = "ac";
const BACKUP_DIR: &str = "leila-backups";
const MANIFEST_FILE: &str = "manifest.json";
const INSTRUCTION_PATH: &str = "./gpt5.5-unrestricted.md";

const FIXED_PACKAGES: &[(&str, &str)] = &[
    ("pefile", "2024.8.26"),
    ("lief", "0.17.6"),
    ("capstone", "5.0.9"),
    ("pyelftools", "0.32"),
    ("xdis", "6.3.0"),
    ("frida", "17.16.4"),
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LeilaDeploymentStatus {
    pub supported: bool,
    pub package_version: String,
    pub platform: String,
    pub architecture: String,
    pub python_version: Option<String>,
    pub python_bits: Option<u32>,
    pub python_modules_installed: bool,
    pub python_module_status: String,
    pub target_codex_home: String,
    pub deployed: bool,
    pub prompt_verified: bool,
    pub identity_verified: bool,
    pub ac_verified: bool,
    pub global_profile_active: bool,
    pub externally_modified: bool,
    pub rollback_available: bool,
    pub last_deployment_at: Option<String>,
    pub last_result: Option<String>,
    pub last_error: Option<String>,
    pub prompt_sha256: Option<String>,
    pub identity_sha256: Option<String>,
    pub ac_sha256: Option<String>,
    pub operation_manifest: Option<String>,
    pub backup_paths: Vec<String>,
    pub logs: Vec<String>,
}

pub type LeilaDeploymentResult = LeilaDeploymentStatus;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct ResourceManifest {
    version: String,
    files: Vec<ResourceFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct ResourceFile {
    path: String,
    sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct BackupRecord {
    name: String,
    target_path: String,
    existed_before: bool,
    before_sha256: Option<String>,
    after_sha256: Option<String>,
    backup_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct DeploymentManifest {
    ccp_version: String,
    leila_version: String,
    target_codex_home: String,
    operation_id: String,
    operated_at: String,
    completed: bool,
    rolled_back: bool,
    original_model_instructions_file: Option<String>,
    config_before_sha256: String,
    config_after_sha256: Option<String>,
    resources: Vec<BackupRecord>,
    stage_results: Vec<String>,
    last_error: Option<String>,
}

#[derive(Debug, Clone)]
struct PythonRuntime {
    executable: String,
    prefix_args: Vec<String>,
    version: String,
    major: u8,
    minor: u8,
    bits: u32,
}

pub fn default_dev_assets_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("apps")
        .join("claude-codex-pro-manager")
        .join("src-tauri")
        .join("resources")
        .join("leila")
        .join("assets")
}

pub fn resolve_assets_root(
    resource_dir: Option<&Path>,
    dev_assets_root: Option<&Path>,
) -> Result<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(root) = resource_dir {
        candidates.push(root.join("resources").join("leila").join("assets"));
        candidates.push(root.join("leila").join("assets"));
        candidates.push(root.to_path_buf());
    }
    if let Some(root) = dev_assets_root {
        candidates.push(root.to_path_buf());
    }
    candidates
        .into_iter()
        .find(|path| validate_assets_root(path).is_ok())
        .ok_or_else(|| anyhow::anyhow!("未找到完整的破甲 {PACKAGE_VERSION} 打包资源"))
}

pub fn package_assets_root(package_root: &Path) -> Result<PathBuf> {
    let extracted = package_root
        .join("resources")
        .join("_extracted")
        .join("assets");
    resolve_assets_root(Some(package_root), Some(&extracted))
}

pub fn inspect_status(target: &Path, resources: Option<&Path>) -> LeilaDeploymentStatus {
    let platform = status_platform().to_string();
    let architecture = status_architecture().to_string();
    let supported = cfg!(windows) && std::env::consts::ARCH == "x86_64";
    let python_runtime = detect_python_runtime();
    let runtime = python_runtime.as_ref().ok();
    let python_modules_installed = runtime
        .as_ref()
        .is_some_and(|runtime| python_modules_match(runtime));
    let mut logs = vec![format!(
        "环境检测开始：{}/{}",
        status_platform(),
        status_architecture()
    )];
    match &python_runtime {
        Ok(runtime) => {
            logs.push(format!(
                "Python 检测：{}，版本 {}，{} 位",
                runtime.executable, runtime.version, runtime.bits
            ));
            logs.push(if python_modules_installed {
                "Python 模块检测：所需版本均已安装".to_string()
            } else {
                "Python 模块检测：待安装或版本不匹配".to_string()
            });
        }
        Err(error) => logs.push(format!("Python 检测失败：{}", error)),
    }
    let mut status = LeilaDeploymentStatus {
        supported,
        package_version: PACKAGE_VERSION.to_string(),
        platform,
        architecture,
        python_version: runtime.as_ref().map(|runtime| runtime.version.clone()),
        python_bits: runtime.as_ref().map(|runtime| runtime.bits),
        python_modules_installed,
        python_module_status: if runtime.is_none() {
            "pending".to_string()
        } else if python_modules_installed {
            "complete".to_string()
        } else {
            "pending".to_string()
        },
        target_codex_home: target.to_string_lossy().to_string(),
        deployed: false,
        prompt_verified: false,
        identity_verified: false,
        ac_verified: false,
        global_profile_active: instruction_path(target).as_deref() == Some(INSTRUCTION_PATH),
        externally_modified: false,
        rollback_available: false,
        last_deployment_at: None,
        last_result: None,
        last_error: None,
        prompt_sha256: None,
        identity_sha256: None,
        ac_sha256: None,
        operation_manifest: None,
        backup_paths: Vec::new(),
        logs,
    };

    if !target.join("config.toml").is_file() {
        status.logs.push("目标目录缺少 config.toml".to_string());
    } else {
        status
            .logs
            .push("Codex 目标目录：已找到 config.toml".to_string());
    }

    if let Some(resources) = resources {
        match validate_assets_root(resources) {
            Ok(resource_manifest) => {
                status.package_version = resource_manifest.version;
                status.logs.push(format!(
                    "打包资源校验：破甲 {}，SHA-256 清单通过",
                    status.package_version
                ));
                status.prompt_sha256 = file_sha256(&resources.join(PROMPT)).ok();
                status.identity_sha256 = tree_sha256(&resources.join(IDENTITY)).ok();
                status.ac_sha256 = tree_sha256(&resources.join(AC)).ok();
                status.prompt_verified = same_hash(&resources.join(PROMPT), &target.join(PROMPT));
                status.identity_verified = same_hash(
                    &resources.join(IDENTITY),
                    &target.join("skills").join(IDENTITY),
                );
                status.ac_verified =
                    same_hash(&resources.join(AC), &target.join("skills").join(AC));
            }
            Err(error) => {
                status.last_error = Some(error.to_string());
                status.logs.push(format!("资源校验失败：{error}"));
            }
        }
    }
    status.deployed = status.prompt_verified
        && status.identity_verified
        && status.ac_verified
        && status.global_profile_active;

    let manifests = read_operation_manifests(target);
    if let Some((path, manifest)) = manifests.first() {
        status.last_deployment_at = Some(manifest.operated_at.clone());
        status.last_result = Some(if manifest.rolled_back {
            "已回滚".to_string()
        } else if manifest.completed {
            "部署成功".to_string()
        } else {
            "部署失败".to_string()
        });
        status.last_error = manifest.last_error.clone().or(status.last_error);
        status.operation_manifest = Some(path.to_string_lossy().to_string());
        status.logs.extend(manifest.stage_results.clone());
    }
    if let Some((path, manifest)) = manifests.iter().find(|(_, manifest)| manifest.completed) {
        if !manifest.rolled_back {
            status.rollback_available = true;
            status.operation_manifest = Some(path.to_string_lossy().to_string());
            if let Some(operation_dir) = path.parent() {
                status.backup_paths = manifest
                    .resources
                    .iter()
                    .filter(|entry| entry.existed_before)
                    .filter_map(|entry| backup_name_for_resource(&entry.name))
                    .map(|name| operation_dir.join(name).to_string_lossy().to_string())
                    .collect();
            }
            status.externally_modified = manifest_changed(target, manifest);
        }
    }
    status
}

pub fn deploy_leila(target: &Path, resources: &Path) -> Result<LeilaDeploymentStatus> {
    deploy_leila_with_logger(target, resources, |_| {})
}

pub fn deploy_leila_with_logger<F>(
    target: &Path,
    resources: &Path,
    mut on_log: F,
) -> Result<LeilaDeploymentStatus>
where
    F: FnMut(&str),
{
    validate_target(target)?;
    let resource_manifest = validate_assets_root(resources)?;
    ensure_supported_environment()?;
    let runtime = detect_python_runtime().context("未找到可用的 Python 3.8-3.14 x64")?;
    validate_python_runtime(&runtime)?;

    let mut logs = vec![
        "部署开始".to_string(),
        format!("检测到 Python {} {} 位", runtime.version, runtime.bits),
    ];
    for line in &logs {
        on_log(line);
    }
    install_python_modules(&runtime, &mut logs, &mut on_log)?;

    deploy_assets_after_prerequisites(target, resources, resource_manifest, logs, &mut on_log)
}

fn deploy_assets_after_prerequisites<F>(
    target: &Path,
    resources: &Path,
    resource_manifest: ResourceManifest,
    logs: Vec<String>,
    on_log: &mut F,
) -> Result<LeilaDeploymentStatus>
where
    F: FnMut(&str),
{
    let mut current = inspect_status(target, Some(resources));
    if current.deployed {
        current.logs = logs;
        if current.rollback_available {
            push_log(
                &mut current.logs,
                on_log,
                "破甲资源和 Codex 配置均为当前版本，无需替换",
            );
        } else {
            push_log(
                &mut current.logs,
                on_log,
                "检测到现有破甲资源和配置已匹配，但没有 CCP 部署清单；本次未替换文件，以避免无法安全回滚的覆盖",
            );
            current.last_result = Some("外部已有匹配资源，未替换".to_string());
        }
        return Ok(current);
    }
    deploy_assets_transaction_with_logger(
        target,
        resources,
        resource_manifest,
        logs,
        || Ok(()),
        on_log,
    )
}

fn deploy_assets_transaction_with_logger<F, L>(
    target: &Path,
    resources: &Path,
    resource_manifest: ResourceManifest,
    logs: Vec<String>,
    after_resources: F,
    on_log: &mut L,
) -> Result<LeilaDeploymentStatus>
where
    F: FnOnce() -> Result<()>,
    L: FnMut(&str),
{
    let operation_dir = create_operation_dir(target)?;
    let manifest_path = operation_dir.join(MANIFEST_FILE);
    let config = target.join("config.toml");
    let original_config = fs::read(&config).context("读取 config.toml 失败")?;
    let original_config_text =
        String::from_utf8(original_config.clone()).context("config.toml 必须使用 UTF-8 编码")?;
    let original_instruction = instruction_path_from_text(&original_config_text);
    let config_backup = operation_dir.join("config.toml.before");
    fs::copy(&config, &config_backup).context("备份 config.toml 失败")?;

    let entries = resource_entries(target, resources);
    let mut records = Vec::new();
    for (name, _, destination, backup_name) in &entries {
        let existed = destination.exists();
        let backup_path = existed.then(|| operation_dir.join(backup_name));
        if let Some(backup) = &backup_path {
            copy_path(destination, backup)
                .with_context(|| format!("备份 {} 失败", destination.display()))?;
        }
        records.push(BackupRecord {
            name: (*name).to_string(),
            target_path: destination.to_string_lossy().to_string(),
            existed_before: existed,
            before_sha256: existed.then(|| tree_sha256(destination).ok()).flatten(),
            after_sha256: None,
            backup_path: backup_path.map(|path| path.to_string_lossy().to_string()),
        });
    }

    let mut operation = DeploymentManifest {
        ccp_version: crate::version::VERSION.to_string(),
        leila_version: resource_manifest.version,
        target_codex_home: target.to_string_lossy().to_string(),
        operation_id: Uuid::new_v4().to_string(),
        operated_at: Utc::now().to_rfc3339(),
        completed: false,
        rolled_back: false,
        original_model_instructions_file: original_instruction,
        config_before_sha256: bytes_sha256(&original_config),
        config_after_sha256: None,
        resources: records,
        stage_results: logs,
        last_error: None,
    };
    write_operation_manifest(&manifest_path, &operation)?;

    let deployment = (|| -> Result<()> {
        for (name, source, destination, _) in &entries {
            if !same_hash(source, destination) {
                replace_path(source, destination)
                    .with_context(|| format!("替换破甲资源 {name} 失败"))?;
                push_log(
                    &mut operation.stage_results,
                    on_log,
                    &format!("已部署资源：{name}"),
                );
            } else {
                push_log(
                    &mut operation.stage_results,
                    on_log,
                    &format!("资源已是当前版本：{name}"),
                );
            }
        }
        after_resources()?;

        let updated_config = set_instruction_path(&original_config_text)?;
        if updated_config.as_bytes() != original_config.as_slice() {
            crate::settings::atomic_write(&config, updated_config.as_bytes())
                .context("写入 config.toml 失败")?;
            push_log(
                &mut operation.stage_results,
                on_log,
                "已更新 model_instructions_file",
            );
        } else {
            push_log(
                &mut operation.stage_results,
                on_log,
                "model_instructions_file 已是目标值",
            );
        }

        for ((_, source, destination, _), record) in
            entries.iter().zip(operation.resources.iter_mut())
        {
            if !same_hash(source, destination) {
                bail!("破甲资源 SHA-256 校验失败：{}", destination.display());
            }
            record.after_sha256 = Some(tree_sha256(destination)?);
        }
        if instruction_path(target).as_deref() != Some(INSTRUCTION_PATH) {
            bail!("Codex 全局指令配置校验失败");
        }
        operation.config_after_sha256 = Some(file_sha256(&config)?);
        push_log(
            &mut operation.stage_results,
            on_log,
            "资源与 Codex 配置 SHA-256 校验通过",
        );
        Ok(())
    })();

    if let Err(error) = deployment {
        let restore_result = restore_from_manifest(target, &operation, &operation_dir);
        operation.last_error = Some(error.to_string());
        push_log(
            &mut operation.stage_results,
            on_log,
            &format!("部署失败：{error}"),
        );
        match restore_result {
            Ok(()) => push_log(
                &mut operation.stage_results,
                on_log,
                "本次资源和配置修改已恢复",
            ),
            Err(restore_error) => push_log(
                &mut operation.stage_results,
                on_log,
                &format!("自动恢复失败：{restore_error}"),
            ),
        }
        let _ = write_operation_manifest(&manifest_path, &operation);
        return Err(error);
    }

    operation.completed = true;
    push_log(&mut operation.stage_results, on_log, "破甲部署完成");
    if let Err(error) = write_operation_manifest(&manifest_path, &operation) {
        operation.completed = false;
        operation.last_error = Some(error.to_string());
        push_log(
            &mut operation.stage_results,
            on_log,
            &format!("写入最终部署清单失败：{error}"),
        );
        let restore_result = restore_from_manifest(target, &operation, &operation_dir);
        match restore_result {
            Ok(()) => push_log(
                &mut operation.stage_results,
                on_log,
                "本次资源和配置修改已恢复",
            ),
            Err(restore_error) => push_log(
                &mut operation.stage_results,
                on_log,
                &format!("自动恢复失败：{restore_error}"),
            ),
        }
        let _ = write_operation_manifest(&manifest_path, &operation);
        return Err(error);
    }
    let mut status = inspect_status(target, Some(resources));
    status.operation_manifest = Some(manifest_path.to_string_lossy().to_string());
    status.backup_paths = operation
        .resources
        .iter()
        .filter_map(|record| record.backup_path.clone())
        .chain(std::iter::once(config_backup.to_string_lossy().to_string()))
        .collect();
    status.logs = operation.stage_results;
    Ok(status)
}

pub fn rollback_leila(target: &Path, resources: Option<&Path>) -> Result<LeilaDeploymentStatus> {
    validate_target(target)?;
    let (manifest_path, mut manifest) = read_operation_manifests(target)
        .into_iter()
        .find(|(_, manifest)| manifest.completed)
        .context("没有可回滚的 CCP 破甲部署")?;
    if manifest.rolled_back {
        bail!("最近一次 CCP 破甲部署已经回滚");
    }
    let operation_dir = manifest_path.parent().context("部署 manifest 路径无效")?;
    restore_from_manifest(target, &manifest, operation_dir)?;
    manifest.rolled_back = true;
    manifest
        .stage_results
        .push("最近一次破甲部署已回滚".to_string());
    write_operation_manifest(&manifest_path, &manifest)?;
    let mut status = inspect_status(target, resources);
    status.logs = manifest.stage_results;
    status.operation_manifest = Some(manifest_path.to_string_lossy().to_string());
    Ok(status)
}

pub fn python_packages_for_version(major: u8, minor: u8) -> Result<Vec<String>> {
    if major != 3 || !(8..=14).contains(&minor) {
        bail!("仅支持 Python 3.8-3.14");
    }
    let androguard = if minor <= 9 { "4.0.1" } else { "4.1.4" };
    let mut packages = FIXED_PACKAGES
        .iter()
        .map(|(name, version)| format!("{name}=={version}"))
        .collect::<Vec<_>>();
    packages.insert(4, format!("androguard=={androguard}"));
    Ok(packages)
}

fn ensure_supported_environment() -> Result<()> {
    if !matches!(std::env::consts::OS, "windows" | "macos")
        || !matches!(std::env::consts::ARCH, "x86_64" | "aarch64")
    {
        bail!("不支持当前破甲部署包，仅支持 Windows/macOS 64 位");
    }
    Ok(())
}

fn status_platform() -> &'static str {
    if cfg!(windows) {
        "win32"
    } else {
        std::env::consts::OS
    }
}

fn status_architecture() -> &'static str {
    if std::env::consts::ARCH == "x86_64" {
        "x64"
    } else {
        std::env::consts::ARCH
    }
}

fn validate_python_runtime(runtime: &PythonRuntime) -> Result<()> {
    python_packages_for_version(runtime.major, runtime.minor)?;
    if runtime.bits != 64 {
        bail!("破甲部署仅支持 64 位 Python");
    }
    Ok(())
}

fn detect_python_runtime() -> Result<PythonRuntime> {
    let candidates = [
        ("python", Vec::<String>::new()),
        ("py", vec!["-3".to_string()]),
        ("python3", Vec::<String>::new()),
    ];
    for (executable, prefix_args) in candidates {
        let mut command = Command::new(executable);
        command.args(&prefix_args).args([
            "-c",
            "import struct,sys; print(f'{sys.version_info.major}.{sys.version_info.minor}|{struct.calcsize(\"P\")*8}')",
        ]);
        hide_window(&mut command);
        let Ok(output) = command.output() else {
            continue;
        };
        if !output.status.success() {
            continue;
        }
        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let Some((version, bits)) = text.split_once('|') else {
            continue;
        };
        let Some((major, minor)) = version.split_once('.') else {
            continue;
        };
        let Ok(major) = major.parse::<u8>() else {
            continue;
        };
        let Ok(minor) = minor.parse::<u8>() else {
            continue;
        };
        let Ok(bits) = bits.parse::<u32>() else {
            continue;
        };
        return Ok(PythonRuntime {
            executable: executable.to_string(),
            prefix_args,
            version: version.to_string(),
            major,
            minor,
            bits,
        });
    }
    bail!("未检测到 Python")
}

fn python_modules_match(runtime: &PythonRuntime) -> bool {
    let Ok(packages) = python_packages_for_version(runtime.major, runtime.minor) else {
        return false;
    };
    let names = packages
        .iter()
        .filter_map(|package| package.split_once("==").map(|(name, _)| name))
        .collect::<Vec<_>>();
    let script = format!(
        "import importlib.metadata as m; print('\\n'.join(f'{{n}}={{m.version(n)}}' for n in {:?}))",
        names
    );
    let Ok(output) = run_python(runtime, &["-c", &script]) else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    let installed = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(name, version)| (name.to_ascii_lowercase(), version.to_string()))
        .collect::<BTreeMap<_, _>>();
    packages.iter().all(|package| {
        package.split_once("==").is_some_and(|(name, version)| {
            installed
                .get(&name.to_ascii_lowercase())
                .is_some_and(|value| value == version)
        })
    })
}

fn install_python_modules<F>(
    runtime: &PythonRuntime,
    logs: &mut Vec<String>,
    on_log: &mut F,
) -> Result<()>
where
    F: FnMut(&str),
{
    if python_modules_match(runtime) {
        push_log(logs, on_log, "Python 模块已是要求版本，跳过 pip 安装");
        return Ok(());
    }
    let packages = python_packages_for_version(runtime.major, runtime.minor)?;
    push_log(
        logs,
        on_log,
        "正在通过动态 pip 源安装 Python 模块（需要网络）",
    );
    let mut invocation = Vec::new();
    invocation.extend(runtime.prefix_args.iter().cloned());
    invocation.extend([
        "-m".to_string(),
        "pip".to_string(),
        "install".to_string(),
        "--disable-pip-version-check".to_string(),
        "--only-binary=:all:".to_string(),
    ]);
    invocation.extend(packages);

    let exit_status =
        run_hidden_powershell_streaming(&runtime.executable, &invocation, &mut |line| {
            push_log(logs, on_log, line);
        })
        .context("启动 PowerShell pip 安装失败")?;
    if !exit_status.success() {
        bail!(
            "Python 模块安装失败（退出码 {:?}）；详见部署日志",
            exit_status.code()
        );
    }
    if !python_modules_match(runtime) {
        bail!("pip 执行成功，但模块版本校验未通过");
    }
    push_log(
        logs,
        on_log,
        &format!("Python {} x64 模块安装完成", runtime.version),
    );
    Ok(())
}

fn push_log<F>(logs: &mut Vec<String>, on_log: &mut F, line: &str)
where
    F: FnMut(&str),
{
    logs.push(line.to_string());
    on_log(line);
}

fn run_python(runtime: &PythonRuntime, args: &[&str]) -> Result<Output> {
    let mut command = Command::new(&runtime.executable);
    command.args(&runtime.prefix_args).args(args);
    hide_window(&mut command);
    command.output().context("执行 Python 失败")
}

#[cfg(windows)]
fn run_hidden_powershell_streaming<F>(
    executable: &str,
    args: &[String],
    on_line: &mut F,
) -> Result<std::process::ExitStatus>
where
    F: FnMut(&str),
{
    let mut tokens = vec![powershell_quote(executable)];
    tokens.extend(args.iter().map(|arg| powershell_quote(arg)));
    let script = format!(
        "$ErrorActionPreference='Stop'; & {}; if ($LASTEXITCODE -ne $null) {{ exit $LASTEXITCODE }}",
        tokens.join(" ")
    );
    let mut command = Command::new("powershell.exe");
    command.args([
        "-NoLogo",
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        &script,
    ]);
    hide_window(&mut command);
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn().context("执行隐藏 PowerShell 失败")?;
    let stdout = child.stdout.take().context("读取 PowerShell stdout 失败")?;
    let stderr = child.stderr.take().context("读取 PowerShell stderr 失败")?;
    let (sender, receiver) = mpsc::channel::<String>();
    let stderr_sender = sender.clone();
    std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines().flatten() {
            let _ = stderr_sender.send(line);
        }
    });
    let stdout_sender = sender.clone();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().flatten() {
            let _ = stdout_sender.send(line);
        }
    });
    drop(sender);
    for line in receiver {
        on_line(&line);
    }
    child.wait().context("等待 PowerShell pip 安装失败")
}

#[cfg(not(windows))]
fn run_hidden_powershell_streaming<F>(
    executable: &str,
    args: &[String],
    on_line: &mut F,
) -> Result<std::process::ExitStatus>
where
    F: FnMut(&str),
{
    let mut command = Command::new(executable);
    command.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn().context("启动 Python pip 安装失败")?;
    let stdout = child.stdout.take().context("读取 Python stdout 失败")?;
    let stderr = child.stderr.take().context("读取 Python stderr 失败")?;
    let (sender, receiver) = mpsc::channel::<String>();
    let streams: [Box<dyn Read + Send>; 2] = [Box::new(stdout), Box::new(stderr)];
    for stream in streams {
        let sender = sender.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stream).lines().flatten() {
                let _ = sender.send(line);
            }
        });
    }
    drop(sender);
    for line in receiver {
        on_line(&line);
    }
    child.wait().context("等待 Python pip 安装失败")
}

fn powershell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn validate_target(target: &Path) -> Result<()> {
    if !target.join("config.toml").is_file() {
        bail!("Codex 目录缺少 config.toml：{}", target.display());
    }
    Ok(())
}

fn validate_assets_root(root: &Path) -> Result<ResourceManifest> {
    for path in [root.join(PROMPT), root.join(IDENTITY), root.join(AC)] {
        if !path.exists() {
            bail!("破甲资源缺失：{}", path.display());
        }
    }
    let manifest_path = root.join(MANIFEST_FILE);
    let manifest: ResourceManifest = serde_json::from_slice(
        &fs::read(&manifest_path)
            .with_context(|| format!("读取资源 manifest 失败：{}", manifest_path.display()))?,
    )
    .context("破甲资源 manifest 格式无效")?;
    if manifest.version != PACKAGE_VERSION {
        bail!("破甲资源版本不匹配：{}", manifest.version);
    }
    if manifest.files.is_empty() {
        bail!("破甲资源 manifest 没有文件记录");
    }
    let expected_files = collect_resource_files(root)?;
    let declared_files = manifest
        .files
        .iter()
        .map(|entry| entry.path.replace('\\', "/"))
        .collect::<BTreeSet<_>>();
    if declared_files != expected_files {
        bail!("破甲资源 manifest 文件列表与打包资源不一致");
    }
    for entry in &manifest.files {
        let relative = Path::new(&entry.path);
        if relative.is_absolute()
            || relative.components().any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            bail!("破甲资源 manifest 包含非法路径：{}", entry.path);
        }
        let actual = file_sha256(&root.join(relative))?;
        if !actual.eq_ignore_ascii_case(&entry.sha256) {
            bail!("破甲资源 SHA-256 不匹配：{}", entry.path);
        }
    }
    Ok(manifest)
}

fn collect_resource_files(root: &Path) -> Result<BTreeSet<String>> {
    let mut files = BTreeSet::new();
    for resource in [root.join(PROMPT), root.join(IDENTITY), root.join(AC)] {
        collect_files(root, &resource, &mut files)?;
    }
    Ok(files)
}

fn collect_files(root: &Path, path: &Path, files: &mut BTreeSet<String>) -> Result<()> {
    if path.is_file() {
        files.insert(
            path.strip_prefix(root)?
                .to_string_lossy()
                .replace('\\', "/"),
        );
        return Ok(());
    }
    for entry in fs::read_dir(path)? {
        collect_files(root, &entry?.path(), files)?;
    }
    Ok(())
}

fn resource_entries(
    target: &Path,
    resources: &Path,
) -> [(&'static str, PathBuf, PathBuf, &'static str); 3] {
    [
        (
            PROMPT,
            resources.join(PROMPT),
            target.join(PROMPT),
            "gpt5.5-unrestricted.md.before",
        ),
        (
            IDENTITY,
            resources.join(IDENTITY),
            target.join("skills").join(IDENTITY),
            "leila-identity.before",
        ),
        (
            AC,
            resources.join(AC),
            target.join("skills").join(AC),
            "ac.before",
        ),
    ]
}

fn create_operation_dir(target: &Path) -> Result<PathBuf> {
    let root = target.join(BACKUP_DIR);
    fs::create_dir_all(&root).context("创建破甲备份目录失败")?;
    let name = format!("{}-{}", Utc::now().format("%Y%m%dT%H%M%SZ"), Uuid::new_v4());
    let directory = root.join(name);
    fs::create_dir(&directory).context("创建破甲操作备份目录失败")?;
    Ok(directory)
}

fn read_operation_manifests(target: &Path) -> Vec<(PathBuf, DeploymentManifest)> {
    let paths = fs::read_dir(target.join(BACKUP_DIR))
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(|entry| entry.ok()))
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| entry.path().join(MANIFEST_FILE))
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    let mut manifests = paths
        .into_iter()
        .filter_map(|path| {
            let manifest: DeploymentManifest = fs::read(&path)
                .ok()
                .and_then(|bytes| serde_json::from_slice(&bytes).ok())?;
            Some((path, manifest))
        })
        .collect::<Vec<_>>();
    manifests.sort_by(|(_, left), (_, right)| {
        let left = chrono::DateTime::parse_from_rfc3339(&left.operated_at).ok();
        let right = chrono::DateTime::parse_from_rfc3339(&right.operated_at).ok();
        right.cmp(&left)
    });
    manifests
}

fn write_operation_manifest(path: &Path, manifest: &DeploymentManifest) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(manifest)?;
    crate::settings::atomic_write(path, &bytes).context("写入破甲部署 manifest 失败")
}

fn restore_from_manifest(
    target: &Path,
    manifest: &DeploymentManifest,
    operation_dir: &Path,
) -> Result<()> {
    let config = target.join("config.toml");
    let config_bytes =
        fs::read(operation_dir.join("config.toml.before")).context("读取 config.toml 备份失败")?;
    crate::settings::atomic_write(&config, &config_bytes).context("恢复 config.toml 失败")?;
    for record in manifest.resources.iter().rev() {
        let destination = target_for_resource(target, &record.name)
            .with_context(|| format!("部署 manifest 包含未知资源：{}", record.name))?;
        if record.existed_before {
            let backup_name =
                backup_name_for_resource(&record.name).context("部署 manifest 包含未知备份资源")?;
            replace_path(&operation_dir.join(backup_name), &destination)
                .with_context(|| format!("恢复 {} 失败", record.name))?;
        } else {
            remove_path(&destination)?;
        }
    }
    Ok(())
}

fn manifest_changed(target: &Path, manifest: &DeploymentManifest) -> bool {
    if manifest.config_after_sha256.as_deref()
        != file_sha256(&target.join("config.toml")).ok().as_deref()
    {
        return true;
    }
    manifest.resources.iter().any(|record| {
        let Some(target_path) = target_for_resource(target, &record.name) else {
            return true;
        };
        record.after_sha256.as_deref() != tree_sha256(&target_path).ok().as_deref()
    })
}

fn target_for_resource(target: &Path, name: &str) -> Option<PathBuf> {
    match name {
        PROMPT => Some(target.join(PROMPT)),
        IDENTITY => Some(target.join("skills").join(IDENTITY)),
        AC => Some(target.join("skills").join(AC)),
        _ => None,
    }
}

fn backup_name_for_resource(name: &str) -> Option<&'static str> {
    match name {
        PROMPT => Some("gpt5.5-unrestricted.md.before"),
        IDENTITY => Some("leila-identity.before"),
        AC => Some("ac.before"),
        _ => None,
    }
}

fn instruction_path(target: &Path) -> Option<String> {
    fs::read_to_string(target.join("config.toml"))
        .ok()
        .and_then(|text| instruction_path_from_text(&text))
}

fn instruction_path_from_text(content: &str) -> Option<String> {
    content
        .parse::<toml_edit::DocumentMut>()
        .ok()
        .and_then(|document| {
            document
                .get("model_instructions_file")?
                .as_str()
                .map(str::to_string)
        })
}

fn set_instruction_path(content: &str) -> Result<String> {
    let mut document = content
        .parse::<toml_edit::DocumentMut>()
        .context("config.toml 解析失败")?;
    document["model_instructions_file"] = toml_edit::value(INSTRUCTION_PATH);
    Ok(document.to_string())
}

fn same_hash(left: &Path, right: &Path) -> bool {
    tree_sha256(left).ok() == tree_sha256(right).ok() && left.exists() && right.exists()
}

fn file_sha256(path: &Path) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("读取文件失败：{}", path.display()))?;
    Ok(bytes_sha256(&bytes))
}

fn bytes_sha256(bytes: &[u8]) -> String {
    let mut hash = Sha256::new();
    hash.update(bytes);
    format!("{:x}", hash.finalize())
}

fn tree_sha256(path: &Path) -> Result<String> {
    if path.is_file() {
        return file_sha256(path);
    }
    if !path.is_dir() {
        bail!("路径不存在：{}", path.display());
    }
    let mut hash = Sha256::new();
    hash_tree(path, path, &mut hash)?;
    Ok(format!("{:x}", hash.finalize()))
}

fn hash_tree(root: &Path, current: &Path, hash: &mut Sha256) -> Result<()> {
    let mut entries = fs::read_dir(current)?.collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            hash_tree(root, &path, hash)?;
        } else {
            let relative = path.strip_prefix(root)?;
            hash.update(relative.to_string_lossy().replace('\\', "/").as_bytes());
            hash.update([0]);
            hash.update(fs::read(path)?);
            hash.update([0]);
        }
    }
    Ok(())
}

fn copy_path(source: &Path, destination: &Path) -> Result<()> {
    if source.is_dir() {
        copy_dir(source, destination)
    } else {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(source, destination)?;
        Ok(())
    }
}

fn copy_dir(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let destination = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &destination)?;
        } else {
            fs::copy(entry.path(), destination)?;
        }
    }
    Ok(())
}

fn replace_path(source: &Path, destination: &Path) -> Result<()> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    let stage = destination.with_extension(format!("leila-stage-{}", Uuid::new_v4()));
    copy_path(source, &stage)?;
    remove_path(destination)?;
    fs::rename(&stage, destination).or_else(|error| {
        let _ = remove_path(&stage);
        Err(error)
    })?;
    Ok(())
}

fn remove_path(path: &Path) -> Result<()> {
    if path.is_dir() {
        fs::remove_dir_all(path)?;
    } else if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn hide_window(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_resources(root: &Path) {
        fs::create_dir_all(root.join(IDENTITY)).unwrap();
        fs::create_dir_all(root.join(AC)).unwrap();
        fs::write(root.join(PROMPT), "SYSTEM\n").unwrap();
        fs::write(root.join(IDENTITY).join("SKILL.md"), "identity\n").unwrap();
        fs::write(root.join(AC).join("SKILL.md"), "ac\n").unwrap();
        let files = [PROMPT, "leila-identity/SKILL.md", "ac/SKILL.md"]
            .into_iter()
            .map(|relative| ResourceFile {
                path: relative.to_string(),
                sha256: file_sha256(&root.join(relative)).unwrap(),
            })
            .collect();
        fs::write(
            root.join(MANIFEST_FILE),
            serde_json::to_vec_pretty(&ResourceManifest {
                version: PACKAGE_VERSION.to_string(),
                files,
            })
            .unwrap(),
        )
        .unwrap();
    }

    fn write_target(root: &Path, config: &str) {
        fs::create_dir_all(root).unwrap();
        fs::write(root.join("config.toml"), config).unwrap();
    }

    fn deploy_assets_for_test(target: &Path, resources: &Path) -> Result<LeilaDeploymentStatus> {
        validate_target(target)?;
        let resource_manifest = validate_assets_root(resources)?;
        deploy_assets_after_prerequisites(
            target,
            resources,
            resource_manifest,
            vec!["test deploy".into()],
            &mut |_| {},
        )
    }

    #[test]
    fn resolves_resource_dir_and_dev_fallback() {
        let root = tempdir().unwrap();
        let release_assets = root.path().join("resources/leila/assets");
        write_resources(&release_assets);
        assert_eq!(
            resolve_assets_root(Some(root.path()), None).unwrap(),
            release_assets
        );
        let dev_root = tempdir().unwrap();
        write_resources(dev_root.path());
        assert_eq!(
            resolve_assets_root(None, Some(dev_root.path())).unwrap(),
            dev_root.path()
        );
        assert!(resolve_assets_root(None, Some(&root.path().join("missing"))).is_err());
    }

    #[test]
    fn bundled_dev_assets_match_manifest() {
        let root = default_dev_assets_root();
        let manifest = validate_assets_root(&root).unwrap();
        assert_eq!(manifest.version, PACKAGE_VERSION);
        assert_eq!(manifest.files.len(), 7);
    }

    #[test]
    fn bundled_assets_deploy_and_rollback_in_temp_codex_home() {
        let root = tempdir().unwrap();
        let target = root.path().join(".codex");
        let resources = default_dev_assets_root();
        let original_config = "model = \"gpt-test\"\nmodel_provider = \"relay-test\"\nmodel_instructions_file = \"old.md\"\n";
        write_target(&target, original_config);
        fs::create_dir_all(target.join("skills/leila-identity")).unwrap();
        fs::create_dir_all(target.join("skills/ac")).unwrap();
        fs::create_dir_all(target.join("hooks")).unwrap();
        fs::write(target.join(PROMPT), "OLD PROMPT\n").unwrap();
        fs::write(
            target.join("skills/leila-identity/SKILL.md"),
            "OLD IDENTITY\n",
        )
        .unwrap();
        fs::write(target.join("skills/ac/SKILL.md"), "OLD AC\n").unwrap();
        fs::write(
            target.join("hooks/instruction-inject.sh"),
            "ORIGINAL HOOK\n",
        )
        .unwrap();
        let hook_before = file_sha256(&target.join("hooks/instruction-inject.sh")).unwrap();

        let deployed = deploy_assets_for_test(&target, &resources).unwrap();
        assert!(deployed.deployed);
        assert!(deployed.prompt_verified && deployed.identity_verified && deployed.ac_verified);
        assert_eq!(deployed.backup_paths.len(), 4);
        assert_eq!(
            deployed.prompt_sha256.as_deref(),
            Some("cbb10f01d5f2b4dda59902c64c94930d9bda55afc221c6f02a9cd296971d9f0d")
        );
        assert_eq!(
            file_sha256(&target.join("hooks/instruction-inject.sh")).unwrap(),
            hook_before
        );
        let deployed_config = fs::read_to_string(target.join("config.toml")).unwrap();
        assert!(deployed_config.contains("model = \"gpt-test\""));
        assert!(deployed_config.contains("model_provider = \"relay-test\""));
        assert!(deployed_config.contains("model_instructions_file = \"./gpt5.5-unrestricted.md\""));

        let rolled_back = rollback_leila(&target, Some(&resources)).unwrap();
        assert_eq!(
            fs::read_to_string(target.join("config.toml")).unwrap(),
            original_config
        );
        assert_eq!(
            fs::read_to_string(target.join(PROMPT)).unwrap(),
            "OLD PROMPT\n"
        );
        assert_eq!(
            fs::read_to_string(target.join("skills/leila-identity/SKILL.md")).unwrap(),
            "OLD IDENTITY\n"
        );
        assert_eq!(
            fs::read_to_string(target.join("skills/ac/SKILL.md")).unwrap(),
            "OLD AC\n"
        );
        assert_eq!(
            file_sha256(&target.join("hooks/instruction-inject.sh")).unwrap(),
            hook_before
        );
        assert!(!rolled_back.rollback_available);
        assert!(target.join(BACKUP_DIR).is_dir());
    }

    #[test]
    fn maps_python_versions_to_expected_androguard() {
        assert!(
            python_packages_for_version(3, 8)
                .unwrap()
                .contains(&"androguard==4.0.1".to_string())
        );
        assert!(
            python_packages_for_version(3, 9)
                .unwrap()
                .contains(&"androguard==4.0.1".to_string())
        );
        assert!(
            python_packages_for_version(3, 10)
                .unwrap()
                .contains(&"androguard==4.1.4".to_string())
        );
        assert!(
            python_packages_for_version(3, 14)
                .unwrap()
                .contains(&"androguard==4.1.4".to_string())
        );
        assert!(python_packages_for_version(3, 15).is_err());
    }

    #[test]
    fn rejects_target_without_config() {
        let root = tempdir().unwrap();
        assert!(validate_target(root.path()).is_err());
    }

    #[test]
    fn deploys_backs_up_and_rolls_back_without_changing_model_or_provider() {
        let root = tempdir().unwrap();
        let target = root.path().join(".codex");
        let resources = root.path().join("assets");
        write_resources(&resources);
        write_target(
            &target,
            "model = \"gpt-test\"\nmodel_provider = \"relay-test\"\nmodel_instructions_file = \"old.md\"\n",
        );
        fs::write(target.join(PROMPT), "OLD\n").unwrap();
        let first = deploy_assets_for_test(&target, &resources).unwrap();
        assert!(first.deployed);
        assert!(first.rollback_available);
        let config = fs::read_to_string(target.join("config.toml")).unwrap();
        assert!(config.contains("model = \"gpt-test\""));
        assert!(config.contains("model_provider = \"relay-test\""));
        assert!(config.contains("model_instructions_file = \"./gpt5.5-unrestricted.md\""));
        assert!(
            first
                .operation_manifest
                .as_deref()
                .is_some_and(|path| Path::new(path).is_file())
        );

        let rolled_back = rollback_leila(&target, Some(&resources)).unwrap();
        let restored = fs::read_to_string(target.join("config.toml")).unwrap();
        assert!(restored.contains("model_instructions_file = \"old.md\""));
        assert_eq!(fs::read_to_string(target.join(PROMPT)).unwrap(), "OLD\n");
        assert!(!target.join("skills/leila-identity").exists());
        assert!(!target.join("skills/ac").exists());
        assert!(!rolled_back.rollback_available);
        assert!(target.join(BACKUP_DIR).is_dir());
    }

    #[test]
    fn detects_external_changes_and_manifest_hash_mismatch() {
        let root = tempdir().unwrap();
        let target = root.path().join(".codex");
        let resources = root.path().join("assets");
        write_resources(&resources);
        write_target(&target, "model = \"gpt-test\"\n");
        deploy_assets_for_test(&target, &resources).unwrap();
        fs::write(target.join("skills/ac/SKILL.md"), "changed\n").unwrap();
        let status = inspect_status(&target, Some(&resources));
        assert!(status.externally_modified);
        assert!(!status.ac_verified);

        let mut manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(resources.join(MANIFEST_FILE)).unwrap()).unwrap();
        manifest["files"][0]["sha256"] = serde_json::Value::String("0".repeat(64));
        fs::write(
            resources.join(MANIFEST_FILE),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        assert!(validate_assets_root(&resources).is_err());
    }

    #[test]
    fn instruction_update_is_idempotent() {
        let source = "model = \"gpt-test\"\nmodel_provider = \"relay-test\"\n";
        let once = set_instruction_path(source).unwrap();
        let twice = set_instruction_path(&once).unwrap();
        assert_eq!(once, twice);
        assert!(once.contains("model = \"gpt-test\""));
        assert!(once.contains("model_provider = \"relay-test\""));
    }

    #[test]
    fn repeated_deploy_does_not_create_another_backup() {
        let root = tempdir().unwrap();
        let target = root.path().join(".codex");
        let resources = root.path().join("assets");
        write_resources(&resources);
        write_target(&target, "model = \"gpt-test\"\n");
        deploy_assets_for_test(&target, &resources).unwrap();
        let backups_before = fs::read_dir(target.join(BACKUP_DIR)).unwrap().count();
        let second = deploy_assets_for_test(&target, &resources).unwrap();
        let backups_after = fs::read_dir(target.join(BACKUP_DIR)).unwrap().count();
        assert_eq!(backups_before, backups_after);
        assert!(second.logs.iter().any(|line| line.contains("无需替换")));
    }

    #[test]
    fn external_matching_resources_are_not_replaced_without_a_rollback_manifest() {
        let root = tempdir().unwrap();
        let target = root.path().join(".codex");
        let resources = root.path().join("assets");
        write_resources(&resources);
        write_target(
            &target,
            "model_instructions_file = \"./gpt5.5-unrestricted.md\"\n",
        );
        fs::create_dir_all(target.join("skills")).unwrap();
        copy_path(&resources.join(PROMPT), &target.join(PROMPT)).unwrap();
        copy_path(
            &resources.join(IDENTITY),
            &target.join("skills").join(IDENTITY),
        )
        .unwrap();
        copy_path(&resources.join(AC), &target.join("skills").join(AC)).unwrap();

        let status = deploy_assets_for_test(&target, &resources).unwrap();

        assert!(status.deployed);
        assert!(!status.rollback_available);
        assert_eq!(
            status.last_result.as_deref(),
            Some("外部已有匹配资源，未替换")
        );
        assert!(
            status
                .logs
                .iter()
                .any(|line| line.contains("没有 CCP 部署清单"))
        );
        assert!(!target.join(BACKUP_DIR).exists());
    }

    #[test]
    fn rollback_does_not_expose_older_completed_deployments() {
        let root = tempdir().unwrap();
        let target = root.path().join(".codex");
        let resources = root.path().join("assets");
        write_resources(&resources);
        write_target(&target, "model = \"gpt-test\"\n");

        deploy_assets_for_test(&target, &resources).unwrap();
        fs::write(target.join("skills/ac/SKILL.md"), "external change\n").unwrap();
        deploy_assets_for_test(&target, &resources).unwrap();

        let rolled_back = rollback_leila(&target, Some(&resources)).unwrap();
        assert!(!rolled_back.rollback_available);
        assert!(rollback_leila(&target, Some(&resources)).is_err());
    }

    #[test]
    fn restore_rejects_manifest_paths_outside_target() {
        let root = tempdir().unwrap();
        let target = root.path().join(".codex");
        let resources = root.path().join("assets");
        write_resources(&resources);
        write_target(&target, "model = \"gpt-test\"\n");
        let status = deploy_assets_for_test(&target, &resources).unwrap();
        let manifest_path = PathBuf::from(status.operation_manifest.unwrap());
        let mut manifest: DeploymentManifest =
            serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
        manifest.resources[0].name = "unknown-resource".to_string();
        manifest.resources[0].target_path = root
            .path()
            .join("outside.txt")
            .to_string_lossy()
            .to_string();
        assert!(
            restore_from_manifest(&target, &manifest, manifest_path.parent().unwrap()).is_err()
        );
        assert!(!root.path().join("outside.txt").exists());
    }

    #[test]
    fn deployment_stage_failure_restores_config_and_resources() {
        let root = tempdir().unwrap();
        let target = root.path().join(".codex");
        let resources = root.path().join("assets");
        write_resources(&resources);
        write_target(
            &target,
            "model = \"gpt-test\"\nmodel_instructions_file = \"old.md\"\n",
        );
        fs::write(target.join(PROMPT), "OLD\n").unwrap();
        let resource_manifest = validate_assets_root(&resources).unwrap();
        let result = deploy_assets_transaction_with_logger(
            &target,
            &resources,
            resource_manifest,
            vec!["test deploy".to_string()],
            || bail!("injected failure"),
            &mut |_| {},
        );
        assert!(result.is_err());
        assert_eq!(
            fs::read_to_string(target.join("config.toml")).unwrap(),
            "model = \"gpt-test\"\nmodel_instructions_file = \"old.md\"\n"
        );
        assert_eq!(fs::read_to_string(target.join(PROMPT)).unwrap(), "OLD\n");
        assert!(!target.join("skills/leila-identity").exists());
        assert!(!target.join("skills/ac").exists());
        let status = inspect_status(&target, Some(&resources));
        assert_eq!(status.last_result.as_deref(), Some("部署失败"));
        assert!(
            status
                .logs
                .iter()
                .any(|line| line.contains("本次资源和配置修改已恢复"))
        );
    }
}
