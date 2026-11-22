use std::process::Child;

pub const MEMORY_LIMIT: u64 = 1536 * 1024 * 1024;

#[cfg(not(windows))]
pub fn attach(_: &Child) -> Result<(), String> {
    // All platforms enforce the cumulative Rust allocation budget in the
    // parser child. Linux adds RLIMIT_AS; Windows adds its Job Object below.
    Ok(())
}

#[cfg(windows)]
pub struct Job(windows_sys::Win32::Foundation::HANDLE);

#[cfg(windows)]
impl Drop for Job {
    fn drop(&mut self) {
        // This private, non-inherited handle also terminates an outstanding
        // parser if the host exits before its normal cleanup completes.
        unsafe { windows_sys::Win32::Foundation::CloseHandle(self.0) };
    }
}

#[cfg(windows)]
pub fn attach(child: &Child) -> Result<Job, String> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::System::JobObjects::*;
    let handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
    if handle.is_null() {
        return Err("Windows could not create an isolated Outlook parser job.".into());
    }
    let job = Job(handle);
    let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
    limits.BasicLimitInformation.LimitFlags =
        JOB_OBJECT_LIMIT_PROCESS_MEMORY | JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    limits.ProcessMemoryLimit = MEMORY_LIMIT as usize;
    let configured = unsafe {
        SetInformationJobObject(
            job.0,
            JobObjectExtendedLimitInformation,
            &limits as *const _ as *const std::ffi::c_void,
            std::mem::size_of_val(&limits) as u32,
        )
    };
    if configured == 0 || unsafe { AssignProcessToJobObject(job.0, child.as_raw_handle()) } == 0 {
        return Err("Windows could not enforce the Outlook parser memory limit. Import was cancelled before reading the PST.".into());
    }
    Ok(job)
}

pub fn in_child() -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let parent = unsafe { libc::getppid() };
        if parent <= 1
            || unsafe { libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) } != 0
            || unsafe { libc::getppid() } != parent
        {
            return Err("The Outlook parser host closed.".into());
        }
        let limit = libc::rlimit {
            rlim_cur: MEMORY_LIMIT as libc::rlim_t,
            rlim_max: MEMORY_LIMIT as libc::rlim_t,
        };
        if unsafe { libc::setrlimit(libc::RLIMIT_AS, &limit) } != 0 {
            return Err("Linux could not enforce the Outlook parser memory limit.".into());
        }
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::JobObjects::*;
        let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
        let queried = unsafe {
            QueryInformationJobObject(
                std::ptr::null_mut(),
                JobObjectExtendedLimitInformation,
                &mut limits as *mut _ as *mut std::ffi::c_void,
                std::mem::size_of_val(&limits) as u32,
                std::ptr::null_mut(),
            )
        };
        if queried == 0
            || limits.BasicLimitInformation.LimitFlags & JOB_OBJECT_LIMIT_PROCESS_MEMORY == 0
            || limits.ProcessMemoryLimit > MEMORY_LIMIT as usize
        {
            return Err("The Outlook parser requires its memory-limited application host.".into());
        }
    }
    Ok(())
}
