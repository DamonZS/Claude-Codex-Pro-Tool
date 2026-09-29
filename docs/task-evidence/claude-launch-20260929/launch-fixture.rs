use std::path::PathBuf;
#[derive(Clone, Debug, PartialEq, Eq)]
enum ClaudeLaunchEntry {
    DirectExecutable(PathBuf),
    PackagedApp(String),
    AppsFolderApp(String),
}

impl ClaudeLaunchEntry {
    fn diagnostic_label(&self) -> &'static str {
        match self {
            Self::DirectExecutable(_) => "桌面程序",
            Self::PackagedApp(_) => "MSIX 包激活",
            Self::AppsFolderApp(_) => "AppsFolder 回退",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct ClaudeLaunchObservation {
    process_ids: Vec<u32>,
    visible_process_id: Option<u32>,
    window_titles: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct ClaudeLaunchOutcome {
    ready: bool,
    process_id: Option<u32>,
    window_titles: Vec<String>,
    diagnostics: Vec<String>,
}

fn execute_claude_launch_plan<L, O, S>(
    plan: &[ClaudeLaunchEntry],
    mut launch: L,
    mut observe: O,
    mut pause: S,
    process_probe_count: usize,
    window_probe_count: usize,
) -> ClaudeLaunchOutcome
where
    L: FnMut(&ClaudeLaunchEntry) -> Result<(), String>,
    O: FnMut() -> ClaudeLaunchObservation,
    S: FnMut(),
{
    let mut diagnostics = Vec::new();
    for entry in plan {
        let label = entry.diagnostic_label();
        let request_accepted = match launch(entry) {
            Ok(()) => true,
            Err(error) => {
                diagnostics.push(format!(
                    "{label}：入口调用失败（{}）",
                    sanitized_claude_launch_error(&error)
                ));
                false
            }
        };

        let mut observation = ClaudeLaunchObservation::default();
        let process_probes = if request_accepted {
            process_probe_count.max(1)
        } else {
            1
        };
        for probe_index in 0..process_probes {
            observation = observe();
            if !observation.process_ids.is_empty() {
                break;
            }
            if probe_index + 1 < process_probes {
                pause();
            }
        }
        if observation.process_ids.is_empty() {
            if request_accepted {
                diagnostics.push(format!("{label}：请求已接受，但未观察到进程"));
            }
            continue;
        }

        let process_id = observation
            .visible_process_id
            .or_else(|| observation.process_ids.first().copied());
        if observation.visible_process_id.is_some() {
            diagnostics.push(format!("{label}：已形成进程和可见窗口"));
            return ClaudeLaunchOutcome {
                ready: true,
                process_id,
                window_titles: observation.window_titles,
                diagnostics,
            };
        }

        for _ in 0..window_probe_count {
            pause();
            observation = observe();
            if let Some(visible_process_id) = observation.visible_process_id {
                diagnostics.push(format!("{label}：已形成进程和可见窗口"));
                return ClaudeLaunchOutcome {
                    ready: true,
                    process_id: Some(visible_process_id),
                    window_titles: observation.window_titles,
                    diagnostics,
                };
            }
        }

        diagnostics.push(format!("{label}：已形成进程，但窗口未就绪"));
        return ClaudeLaunchOutcome {
            ready: false,
            process_id,
            window_titles: observation.window_titles,
            diagnostics,
        };
    }

    ClaudeLaunchOutcome {
        diagnostics,
        ..ClaudeLaunchOutcome::default()
    }
}

fn sanitized_claude_launch_error(error: &str) -> String {
    let lowered = error.to_ascii_lowercase();
    if lowered.contains("0x80070005")
        || lowered.contains("80070005")
        || lowered.contains("access is denied")
        || lowered.contains("access denied")
        || lowered.contains("拒绝访问")
    {
        "系统拒绝（HRESULT 0x80070005）".to_string()
    } else if lowered.contains("not found")
        || lowered.contains("cannot find")
        || lowered.contains("找不到")
    {
        "入口不存在".to_string()
    } else {
        "系统调用失败".to_string()
    }
}

fn main() {
    let launches = std::cell::Cell::new(0);
    let recoveries = std::cell::Cell::new(0);
    let plan = vec![ClaudeLaunchEntry::PackagedApp("Claude_family!Claude".into()),
        ClaudeLaunchEntry::AppsFolderApp("Claude_family!Claude".into())];
    let outcome = execute_claude_launch_plan(&plan, |entry| {
        launches.set(launches.get() + 1);
        match entry { ClaudeLaunchEntry::PackagedApp(_) => Err("HRESULT 0x80070005".into()), _ => Ok(()) }
    }, ClaudeLaunchObservation::default, || {}, 1, 0);
    let outcome = outcome;
    println!("ready={} process={:?} recovery_calls={} launches={}",
        outcome.ready, outcome.process_id, recoveries.get(), launches.get());
}