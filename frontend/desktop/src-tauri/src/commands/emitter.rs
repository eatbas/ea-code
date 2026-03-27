use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::task::JoinHandle;

pub(crate) fn emit_done(app: &AppHandle, done_event: &str) {
    let _ = app.emit(done_event, ());
}

pub(crate) fn emit_items<T, I>(app: &AppHandle, item_event: &str, items: I)
where
    T: Serialize,
    I: IntoIterator<Item = T>,
{
    for item in items {
        let _ = app.emit(item_event, &item);
    }
}

pub(crate) fn spawn_joined_task_emits(
    app: AppHandle,
    done_event: &'static str,
    handles: Vec<JoinHandle<()>>,
) {
    tokio::spawn(async move {
        for handle in handles {
            let _ = handle.await;
        }
        emit_done(&app, done_event);
    });
}
