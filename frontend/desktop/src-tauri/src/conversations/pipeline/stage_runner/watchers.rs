use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::time::{sleep, Duration};

pub(super) struct StageWatchers {
    pub file_ready: Arc<AtomicBool>,
    pub local_stop: Arc<AtomicBool>,
}

pub(super) fn spawn_stage_watchers(
    file_to_watch: String,
    abort: Arc<AtomicBool>,
) -> StageWatchers {
    let file_ready = Arc::new(AtomicBool::new(false));
    let local_stop = Arc::new(AtomicBool::new(false));

    {
        let file_ready_handle = file_ready.clone();
        let local_stop_handle = local_stop.clone();
        let abort_handle = abort.clone();
        let file_path = file_to_watch.clone();

        tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(2)).await;
                if abort_handle.load(Ordering::Acquire) {
                    return;
                }
                if Path::new(&file_path).exists() {
                    sleep(Duration::from_secs(3)).await;
                    file_ready_handle.store(true, Ordering::Release);
                    local_stop_handle.store(true, Ordering::Release);
                    return;
                }
            }
        });
    }

    {
        let local_stop_handle = local_stop.clone();
        let abort_handle = abort.clone();

        tokio::spawn(async move {
            while !local_stop_handle.load(Ordering::Acquire) {
                if abort_handle.load(Ordering::Acquire) {
                    local_stop_handle.store(true, Ordering::Release);
                    return;
                }
                sleep(Duration::from_millis(200)).await;
            }
        });
    }

    StageWatchers {
        file_ready,
        local_stop,
    }
}
