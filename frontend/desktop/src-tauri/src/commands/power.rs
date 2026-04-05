/// Keep-awake commands — prevent the system from sleeping whilst the app is open.

#[cfg(target_os = "macos")]
mod platform {
    use std::process::Child;
    use std::sync::Mutex;

    static CHILD: Mutex<Option<Child>> = Mutex::new(None);

    pub fn enable() -> Result<(), String> {
        let mut guard = CHILD.lock().map_err(|_| "Lock poisoned".to_string())?;
        if guard.is_some() {
            return Ok(());
        }
        let child = std::process::Command::new("caffeinate")
            .arg("-i")
            .spawn()
            .map_err(|e| format!("Failed to start caffeinate: {e}"))?;
        *guard = Some(child);
        Ok(())
    }

    pub fn disable() -> Result<(), String> {
        let mut guard = CHILD.lock().map_err(|_| "Lock poisoned".to_string())?;
        if let Some(mut child) = guard.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        Ok(())
    }
}

#[cfg(target_os = "windows")]
mod platform {
    pub fn enable() -> Result<(), String> {
        // ES_CONTINUOUS | ES_SYSTEM_REQUIRED
        unsafe {
            windows_sys::Win32::System::Power::SetThreadExecutionState(0x80000001 | 0x00000001);
        }
        Ok(())
    }

    pub fn disable() -> Result<(), String> {
        // ES_CONTINUOUS only — clears the previous flags
        unsafe {
            windows_sys::Win32::System::Power::SetThreadExecutionState(0x80000000);
        }
        Ok(())
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod platform {
    pub fn enable() -> Result<(), String> {
        Ok(())
    }

    pub fn disable() -> Result<(), String> {
        Ok(())
    }
}

#[tauri::command]
pub fn enable_keep_awake() -> Result<(), String> {
    platform::enable()
}

#[tauri::command]
pub fn disable_keep_awake() -> Result<(), String> {
    platform::disable()
}
