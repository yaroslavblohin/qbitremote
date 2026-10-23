use windows_sys::Win32::{
    Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE},
    System::Threading::CreateMutexW,
};

// The named handle lives until after eframe has saved settings and shut down.
// Windows also releases it if the process crashes; no stale lock file is left.
pub struct InstanceGuard(HANDLE);

impl InstanceGuard {
    pub fn acquire() -> std::io::Result<Option<Self>> {
        let name = if cfg!(feature = "ui-smoke") {
            "Local\\qBitRemote.UI-Smoke.26B183CC"
        } else {
            "Local\\qBitRemote.Application.26B183CC"
        };
        Self::named(name)
    }

    fn named(name: &str) -> std::io::Result<Option<Self>> {
        let name: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
        // The name is NUL-terminated; the default security descriptor is used.
        // Existence is checked atomically by Windows, before any UI/thread starts.
        let (handle, error) = unsafe {
            let handle = CreateMutexW(std::ptr::null(), 0, name.as_ptr());
            (handle, GetLastError())
        };
        if handle.is_null() {
            return Err(std::io::Error::from_raw_os_error(error as i32));
        }
        let guard = Self(handle);
        if error == ERROR_ALREADY_EXISTS {
            drop(guard);
            Ok(None)
        } else {
            Ok(Some(guard))
        }
    }
}

impl Drop for InstanceGuard {
    fn drop(&mut self) {
        // Owns exactly the handle returned by CreateMutexW; no mutex ownership
        // was requested, so ReleaseMutex is unnecessary.
        unsafe {
            CloseHandle(self.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::windows::process::CommandExt;

    #[test]
    fn blocks_another_process_and_allows_restart() {
        let name = format!("Local\\qBitRemote.Test.{}", std::process::id());
        let guard = InstanceGuard::named(&name).unwrap().unwrap();
        let probe = |expected| {
            let result = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "single_instance::tests::child_probe",
                    "--ignored",
                ])
                .env("QBIT_INSTANCE_TEST_NAME", &name)
                .env("QBIT_INSTANCE_TEST_EXPECTED", expected)
                .creation_flags(0x08000000)
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stdout)
            );
        };
        probe("blocked");
        // The rejected process must not release the original process's lock.
        probe("blocked");
        drop(guard);
        probe("acquired");
        probe("acquired");
    }

    #[test]
    #[ignore = "launched by the cross-process single-instance test"]
    fn child_probe() {
        let name = std::env::var("QBIT_INSTANCE_TEST_NAME").unwrap();
        let expected = std::env::var("QBIT_INSTANCE_TEST_EXPECTED").unwrap();
        let guard = InstanceGuard::named(&name).unwrap();
        assert_eq!(guard.is_some(), expected == "acquired");
    }
}
