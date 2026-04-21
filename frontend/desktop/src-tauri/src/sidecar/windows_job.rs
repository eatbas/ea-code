#![cfg(target_os = "windows")]

//! Windows Job Object that owns every descendant of the Maestro process.
//!
//! `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` is the only reliable way to guarantee
//! the Python sidecar and its bash/CLI descendants are killed when Maestro
//! exits — regardless of whether the exit was clean, a crash, a hard taskkill,
//! or a power loss. The handle is held for the lifetime of the process; when
//! it drops (or the process dies), the kernel tree-kills the whole job.

use std::sync::OnceLock;

use windows_sys::Win32::Foundation::{CloseHandle, FALSE, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
    SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};
use windows_sys::Win32::System::Threading::{
    OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE,
};

/// `HANDLE` is a raw pointer type on Windows. Wrapping it here lets us store
/// the job in a `OnceLock` that is `Send + Sync` — safe because we never
/// mutate the handle after creation and Win32 Job APIs are thread-safe.
struct JobHandle(HANDLE);

unsafe impl Send for JobHandle {}
unsafe impl Sync for JobHandle {}

static JOB: OnceLock<Option<JobHandle>> = OnceLock::new();

/// Lazily create the process-wide Job Object. Returns `None` if creation
/// failed — callers should fall back to best-effort `taskkill` cleanup.
fn job_handle() -> Option<HANDLE> {
    JOB.get_or_init(|| unsafe {
        let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
        if job.is_null() || job == INVALID_HANDLE_VALUE {
            eprintln!("[sidecar] CreateJobObjectW failed; descendants will not be auto-killed");
            return None;
        }

        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

        let ok = SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &info as *const _ as *const _,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        );
        if ok == 0 {
            eprintln!("[sidecar] SetInformationJobObject failed; closing job handle");
            CloseHandle(job);
            return None;
        }

        Some(JobHandle(job))
    })
    .as_ref()
    .map(|h| h.0)
}

/// Attach the given PID to the process-wide job so the kernel kills it — and
/// anything it spawns — when Maestro exits. Best-effort: failures are logged
/// but do not abort sidecar startup, since the async `taskkill` path still
/// covers clean shutdown.
pub fn attach_pid(pid: u32) {
    let Some(job) = job_handle() else {
        return;
    };

    unsafe {
        let process = OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, FALSE, pid);
        if process.is_null() {
            eprintln!("[sidecar] OpenProcess({pid}) failed; cannot attach to job");
            return;
        }

        let ok = AssignProcessToJobObject(job, process);
        if ok == 0 {
            eprintln!(
                "[sidecar] AssignProcessToJobObject({pid}) failed; descendants may survive crash"
            );
        }

        CloseHandle(process);
    }
}
