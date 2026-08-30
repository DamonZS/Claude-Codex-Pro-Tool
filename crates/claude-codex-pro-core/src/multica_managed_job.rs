#[cfg(windows)]
mod windows_job {
    use std::mem::size_of;
    use std::os::windows::io::AsRawHandle;
    use std::process::Child;

    use anyhow::{Context, anyhow};
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
        SetInformationJobObject, TerminateJobObject,
    };
    use windows::core::PCWSTR;

    /// Owns the private Job Object used by the fixed managed Multica daemon.
    /// Closing the final handle terminates every process assigned to the job.
    pub(crate) struct ManagedProcessJob {
        handle: HANDLE,
    }

    // Windows kernel handles are valid across threads. Access remains scoped
    // to the sidecar registry mutex, and this type is the sole closing owner.
    unsafe impl Send for ManagedProcessJob {}

    impl ManagedProcessJob {
        pub(crate) fn assign(child: &Child) -> anyhow::Result<Self> {
            let handle = unsafe { CreateJobObjectW(None, PCWSTR::null()) }
                .map_err(|_| anyhow!("managed_runtime_job_create_failed"))?;
            let job = Self { handle };

            let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            unsafe {
                SetInformationJobObject(
                    job.handle,
                    JobObjectExtendedLimitInformation,
                    (&raw const limits).cast(),
                    u32::try_from(size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>())
                        .expect("Job Object information size fits in u32"),
                )
            }
            .map_err(|_| anyhow!("managed_runtime_job_configure_failed"))?;

            let process_handle = HANDLE(child.as_raw_handle());
            unsafe { AssignProcessToJobObject(job.handle, process_handle) }
                .map_err(|_| anyhow!("managed_runtime_job_assign_failed"))?;
            Ok(job)
        }

        pub(crate) fn terminate(&self) -> anyhow::Result<()> {
            unsafe { TerminateJobObject(self.handle, 1) }
                .context("managed_runtime_job_terminate_failed")
        }
    }

    impl Drop for ManagedProcessJob {
        fn drop(&mut self) {
            let _ = unsafe { CloseHandle(self.handle) };
        }
    }

    #[cfg(test)]
    mod tests {
        use std::process::{Child, Command, Stdio};
        use std::time::{Duration, Instant};

        use super::ManagedProcessJob;

        struct TestChild(Child);

        impl Drop for TestChild {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }

        fn sleeping_child() -> TestChild {
            use std::os::windows::process::CommandExt;

            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            TestChild(
                Command::new("ping.exe")
                    .args(["-t", "127.0.0.1"])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .creation_flags(CREATE_NO_WINDOW)
                    .spawn()
                    .expect("spawn Windows process fixture"),
            )
        }

        fn wait_for_exit(child: &mut Child, timeout: Duration) -> bool {
            let deadline = Instant::now() + timeout;
            loop {
                match child.try_wait() {
                    Ok(Some(_)) => return true,
                    Ok(None) if Instant::now() < deadline => {
                        std::thread::sleep(Duration::from_millis(20));
                    }
                    _ => return false,
                }
            }
        }

        #[test]
        fn managed_job_terminates_only_the_assigned_process() {
            let mut managed = sleeping_child();
            let mut unrelated = sleeping_child();
            let job = ManagedProcessJob::assign(&managed.0).expect("assign managed fixture");

            assert!(managed.0.try_wait().unwrap().is_none());
            assert!(unrelated.0.try_wait().unwrap().is_none());
            job.terminate().expect("terminate managed job");

            assert!(wait_for_exit(&mut managed.0, Duration::from_secs(3)));
            assert!(
                unrelated.0.try_wait().unwrap().is_none(),
                "a process outside the managed Job Object must remain alive"
            );
        }

        #[test]
        fn managed_job_close_kills_the_assigned_process() {
            let mut managed = sleeping_child();
            let mut unrelated = sleeping_child();
            let job = ManagedProcessJob::assign(&managed.0).expect("assign managed fixture");

            drop(job);

            assert!(wait_for_exit(&mut managed.0, Duration::from_secs(3)));
            assert!(
                unrelated.0.try_wait().unwrap().is_none(),
                "closing the managed job must not affect an unrelated process"
            );
        }
    }
}

#[cfg(windows)]
pub(crate) use windows_job::ManagedProcessJob;
