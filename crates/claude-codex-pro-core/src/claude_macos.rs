use std::path::{Path, PathBuf};
#[cfg(target_os = "macos")]
use std::process::Command;

const CLAUDE_BUNDLE_IDS: &[&str] = &["com.anthropic.claudefordesktop", "com.anthropic.claude"];
#[cfg(any(target_os = "macos", test))]
const CLAUDE_APP_NAMES: &[&str] = &["Claude.app", "Claude Desktop.app", "Anthropic Claude.app"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ClaudeMacosBundle {
    pub bundle_path: PathBuf,
    pub executable_path: PathBuf,
    pub bundle_id: String,
}

#[cfg(any(target_os = "macos", test))]
pub(crate) fn candidate_paths_from_sources(
    home: Option<&Path>,
    process_output: &str,
    spotlight_output: &str,
) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    for (_, executable) in parse_process_output(process_output) {
        if let Some(bundle) = app_ancestor(&executable) {
            push_unique(&mut candidates, bundle);
        }
    }
    let mut roots = Vec::new();
    if let Some(home) = home {
        roots.push(home.join("Applications"));
    }
    roots.push(PathBuf::from("/Applications"));
    for root in roots {
        for name in CLAUDE_APP_NAMES {
            push_unique(&mut candidates, root.join(name));
        }
    }
    for line in spotlight_output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let path = PathBuf::from(line);
        if let Some(bundle) = app_ancestor(&path) {
            push_unique(&mut candidates, bundle);
        }
    }
    candidates
}

pub(crate) fn resolve_bundle_path(path: &Path) -> Option<ClaudeMacosBundle> {
    let bundle_path = app_ancestor(path)?;
    if is_app_translocation_path(&bundle_path) {
        return None;
    }
    let plist = bundle_path.join("Contents").join("Info.plist");
    if !plist.is_file() {
        return None;
    }
    let bundle_id = plist_value(&plist, "CFBundleIdentifier")?;
    if !CLAUDE_BUNDLE_IDS
        .iter()
        .any(|known| known.eq_ignore_ascii_case(&bundle_id))
    {
        return None;
    }
    let executable_name = plist_value(&plist, "CFBundleExecutable")?;
    if executable_name.contains(['/', '\\']) || executable_name.trim().is_empty() {
        return None;
    }
    let executable_path = bundle_path
        .join("Contents")
        .join("MacOS")
        .join(executable_name);
    if !executable_path.is_file() || !is_executable(&executable_path) {
        return None;
    }
    Some(ClaudeMacosBundle {
        bundle_path,
        executable_path,
        bundle_id,
    })
}

#[cfg(target_os = "macos")]
pub(crate) fn discover_bundle_default() -> Option<ClaudeMacosBundle> {
    let process_output = command_stdout(Command::new("ps").args(["-axo", "pid=,comm="]));
    let spotlight_output = spotlight_bundle_paths();
    let home = directories::BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf());
    candidate_paths_from_sources(home.as_deref(), &process_output, &spotlight_output)
        .into_iter()
        .find_map(|path| resolve_bundle_path(&path))
}

#[cfg(target_os = "macos")]
pub(crate) fn running_processes_default() -> Vec<(u32, ClaudeMacosBundle)> {
    let output = command_stdout(Command::new("ps").args(["-axo", "pid=,comm="]));
    parse_process_output(&output)
        .into_iter()
        .filter_map(|(pid, executable)| {
            resolve_bundle_path(&executable).map(|bundle| (pid, bundle))
        })
        .collect()
}

#[cfg(target_os = "macos")]
pub(crate) fn terminate_process(process_id: u32) -> bool {
    Command::new("kill")
        .arg("-TERM")
        .arg(process_id.to_string())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(any(target_os = "macos", test))]
fn parse_process_output(output: &str) -> Vec<(u32, PathBuf)> {
    output
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let split = line.find(char::is_whitespace)?;
            let pid = line[..split].parse().ok()?;
            let executable = line[split..].trim();
            (!executable.is_empty()).then(|| (pid, PathBuf::from(executable)))
        })
        .collect()
}

fn app_ancestor(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .find(|ancestor| {
            ancestor
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case("app"))
        })
        .map(Path::to_path_buf)
}

fn is_app_translocation_path(path: &Path) -> bool {
    path.to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase()
        .contains("/apptranslocation/")
}

fn plist_value(path: &Path, key: &str) -> Option<String> {
    if let Ok(text) = std::fs::read_to_string(path)
        && let Some(value) = plist_xml_string_value(&text, key)
    {
        return Some(value);
    }
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("/usr/bin/plutil")
            .args(["-extract", key, "raw", "-o", "-"])
            .arg(path)
            .output()
            .ok()?;
        if output.status.success() {
            let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
            return (!value.is_empty()).then_some(value);
        }
    }
    None
}

fn plist_xml_string_value(plist: &str, key: &str) -> Option<String> {
    let (_, rest) = plist.split_once(&format!("<key>{key}</key>"))?;
    let (_, rest) = rest.split_once("<string>")?;
    let (value, _) = rest.split_once("</string>")?;
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

#[cfg(any(target_os = "macos", test))]
fn push_unique(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if !paths.iter().any(|existing| existing == &path) {
        paths.push(path);
    }
}

#[cfg(target_os = "macos")]
fn spotlight_bundle_paths() -> String {
    let mut paths = String::new();
    for bundle_id in CLAUDE_BUNDLE_IDS {
        let query = format!("kMDItemCFBundleIdentifier == '{bundle_id}'");
        let output = command_stdout(Command::new("mdfind").arg(query));
        if !output.is_empty() {
            paths.push_str(&output);
            paths.push('\n');
        }
    }
    paths
}

#[cfg(target_os = "macos")]
fn command_stdout(command: &mut Command) -> String {
    command
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_bundle(root: &Path, bundle_id: &str, executable: &str) -> PathBuf {
        let bundle = root.join("Claude.app");
        let macos = bundle.join("Contents").join("MacOS");
        std::fs::create_dir_all(&macos).unwrap();
        std::fs::create_dir_all(bundle.join("Contents").join("Resources")).unwrap();
        std::fs::write(
            bundle.join("Contents").join("Info.plist"),
            format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>{bundle_id}</string>
<key>CFBundleExecutable</key><string>{executable}</string>
</dict></plist>"#
            ),
        )
        .unwrap();
        std::fs::write(macos.join(executable), b"binary").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(
                macos.join(executable),
                std::fs::Permissions::from_mode(0o755),
            )
            .unwrap();
        }
        bundle
    }

    #[test]
    fn candidates_cover_system_user_running_and_bundle_id_sources() {
        let home = Path::new("/Users/tester");
        let process = "42 /Users/tester/Applications/Claude.app/Contents/MacOS/Claude\n";
        let spotlight = "/Volumes/Tools/Claude.app\n";

        let candidates = candidate_paths_from_sources(Some(home), process, spotlight);

        assert_eq!(
            candidates.first(),
            Some(&home.join("Applications").join("Claude.app"))
        );
        assert!(candidates.contains(&PathBuf::from("/Applications/Claude.app")));
        assert!(candidates.contains(&PathBuf::from("/Volumes/Tools/Claude.app")));
    }

    #[test]
    fn resolves_valid_bundle_from_bundle_or_executable_path() {
        let temp = tempfile::tempdir().unwrap();
        let bundle = create_bundle(temp.path(), "com.anthropic.claudefordesktop", "Claude");

        let from_bundle = resolve_bundle_path(&bundle).unwrap();
        let from_executable =
            resolve_bundle_path(&bundle.join("Contents").join("MacOS").join("Claude")).unwrap();

        assert_eq!(from_bundle, from_executable);
        assert_eq!(from_bundle.bundle_path, bundle);
        assert_eq!(from_bundle.bundle_id, "com.anthropic.claudefordesktop");
    }

    #[test]
    fn rejects_wrong_bundle_id_missing_executable_and_translocation() {
        let temp = tempfile::tempdir().unwrap();
        let wrong = create_bundle(temp.path(), "com.example.Claude", "Claude");
        assert!(resolve_bundle_path(&wrong).is_none());

        let missing_root = temp.path().join("missing");
        let missing = create_bundle(&missing_root, "com.anthropic.claudefordesktop", "Claude");
        std::fs::remove_file(missing.join("Contents/MacOS/Claude")).unwrap();
        assert!(resolve_bundle_path(&missing).is_none());

        let translocated = temp
            .path()
            .join("AppTranslocation")
            .join("random")
            .join("d");
        let translocated = create_bundle(&translocated, "com.anthropic.claudefordesktop", "Claude");
        assert!(resolve_bundle_path(&translocated).is_none());
    }
}
