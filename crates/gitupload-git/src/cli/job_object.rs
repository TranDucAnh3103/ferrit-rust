//! Windows Job Object process tree wrapper ensuring zero orphaned `git.exe` / `ssh.exe` subprocesses on cancellation.

#![allow(unsafe_code)]

use std::io;

#[cfg(windows)]
use std::os::windows::io::RawHandle;
#[cfg(windows)]
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
#[cfg(windows)]
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};

/// RAII wrapper around a Win32 Job Object configured with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`.
/// Dropping or terminating the handle atomically destroys all assigned child processes.
pub struct WindowsJobObject {
    #[cfg(windows)]
    handle: HANDLE,
    #[cfg(not(windows))]
    _dummy: (),
}

// SAFETY: Win32 Job Object kernel handles can be safely transferred and dropped across threads.
unsafe impl Send for WindowsJobObject {}
// SAFETY: Operations on the handle are kernel-synchronized or owned via `&mut self` / `Drop`.
unsafe impl Sync for WindowsJobObject {}

impl WindowsJobObject {
    /// Creates an anonymous Windows Job Object with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`.
    pub fn new() -> io::Result<Self> {
        #[cfg(windows)]
        {
            // SAFETY: Passing null security attributes and null name creates an unnamed job object
            // owned by the current process; errors are checked via `.is_null()`.
            let handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
            if handle.is_null() {
                return Err(io::Error::last_os_error());
            }

            // SAFETY: Zero-initializing POD Win32 struct `JOBOBJECT_EXTENDED_LIMIT_INFORMATION`.
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

            // SAFETY: `handle` is a valid non-null job object handle and `info` matches the expected size.
            let ret = unsafe {
                SetInformationJobObject(
                    handle,
                    JobObjectExtendedLimitInformation,
                    (&raw const info).cast(),
                    std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                )
            };

            if ret == 0 {
                let err = io::Error::last_os_error();
                // SAFETY: `handle` is valid and non-null, closing before returning error.
                unsafe {
                    CloseHandle(handle);
                }
                return Err(err);
            }

            Ok(Self { handle })
        }
        #[cfg(not(windows))]
        {
            Ok(Self { _dummy: () })
        }
    }

    /// Assigns an active child process handle to this Job Object.
    #[cfg(windows)]
    pub fn assign_process(&self, process_handle: RawHandle) -> io::Result<()> {
        if self.handle.is_null() {
            return Err(io::Error::other("Job object handle already closed"));
        }
        // SAFETY: `self.handle` is a valid job object and `process_handle` is an open process handle.
        let ret = unsafe { AssignProcessToJobObject(self.handle, process_handle as HANDLE) };
        if ret == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    /// Terminates every process in the job immediately and closes the handle.
    pub fn terminate(&mut self) -> io::Result<()> {
        #[cfg(windows)]
        {
            if !self.handle.is_null() {
                // SAFETY: `self.handle` is a valid job object; exit code 1 marks aborted job.
                let term_ok = unsafe { TerminateJobObject(self.handle, 1) };
                let err = if term_ok == 0 {
                    Some(io::Error::last_os_error())
                } else {
                    None
                };
                // SAFETY: Closing owned kernel handle and zeroing to prevent double-close.
                unsafe {
                    CloseHandle(self.handle);
                }
                self.handle = std::ptr::null_mut();
                if let Some(e) = err {
                    return Err(e);
                }
            }
        }
        Ok(())
    }
}

impl Drop for WindowsJobObject {
    fn drop(&mut self) {
        #[cfg(windows)]
        {
            if !self.handle.is_null() {
                // SAFETY: `CloseHandle` triggers `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` and frees kernel handle.
                unsafe {
                    CloseHandle(self.handle);
                }
                self.handle = std::ptr::null_mut();
            }
        }
    }
}
