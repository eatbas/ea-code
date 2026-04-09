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
    use windows_sys::Win32::System::Power::{
        SetThreadExecutionState, ES_CONTINUOUS, ES_SYSTEM_REQUIRED,
    };

    pub fn enable() -> Result<(), String> {
        let previous_state = unsafe { SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED) };

        if previous_state == 0 {
            return Err("Failed to enable Windows keep-awake state.".to_string());
        }

        Ok(())
    }

    pub fn disable() -> Result<(), String> {
        let previous_state = unsafe { SetThreadExecutionState(ES_CONTINUOUS) };

        if previous_state == 0 {
            return Err("Failed to clear Windows keep-awake state.".to_string());
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
