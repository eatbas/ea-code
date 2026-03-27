use tokio::process::Command;

pub(crate) async fn kill_orphaned_hive_api(port: u16) {
    let port_str = port.to_string();

    #[cfg(not(target_os = "windows"))]
    {
        let output = Command::new("lsof")
            .args(["-ti", &format!("tcp:{port_str}")])
            .output()
            .await;

        if let Ok(output) = output {
            let pids = String::from_utf8_lossy(&output.stdout);
            for pid_str in pids.split_whitespace() {
                if let Ok(pid) = pid_str.parse::<i32>() {
                    eprintln!("[sidecar] Killing orphaned process {pid} on port {port_str}");
                    unsafe {
                        libc::kill(pid, libc::SIGKILL);
                    }
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        let output = Command::new("cmd")
            .args(["/C", &format!("netstat -ano | findstr :{port_str} | findstr LISTENING")])
            .output()
            .await;

        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut killed = std::collections::HashSet::new();
            for line in stdout.lines() {
                if let Some(pid_str) = line.split_whitespace().last() {
                    if killed.contains(pid_str) {
                        continue;
                    }
                    eprintln!("[sidecar] Killing orphaned process {pid_str} on port {port_str}");
                    let _ = Command::new("taskkill")
                        .args(["/T", "/F", "/PID", pid_str])
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .status()
                        .await;
                    killed.insert(pid_str.to_string());
                }
            }
        }
    }
}
