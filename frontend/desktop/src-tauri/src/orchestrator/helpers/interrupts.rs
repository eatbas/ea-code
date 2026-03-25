//! Pipeline cancellation, pause, and interrupt handling.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::models::*;

pub fn is_cancelled(cancel_flag: &Arc<AtomicBool>) -> bool {
    cancel_flag.load(Ordering::SeqCst)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunInterrupt {
    Pause,
    Cancel,
}

pub async fn wait_for_interrupt(
    pause_flag: &Arc<AtomicBool>,
    cancel_flag: &Arc<AtomicBool>,
) -> RunInterrupt {
    loop {
        if cancel_flag.load(Ordering::SeqCst) {
            return RunInterrupt::Cancel;
        }
        if pause_flag.load(Ordering::SeqCst) {
            return RunInterrupt::Pause;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
}

pub async fn wait_if_paused(pause_flag: &Arc<AtomicBool>, cancel_flag: &Arc<AtomicBool>) -> bool {
    while pause_flag.load(Ordering::SeqCst) {
        if cancel_flag.load(Ordering::SeqCst) {
            return true;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    false
}

pub async fn wait_for_cancel(cancel_flag: &Arc<AtomicBool>) {
    loop {
        if cancel_flag.load(Ordering::SeqCst) {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
}

pub fn push_cancel_iteration(run: &mut PipelineRun, iter_num: u32, stages: Vec<StageResult>) {
    run.iterations.push(Iteration {
        number: iter_num,
        stages,
        verdict: None,
        judge_reasoning: None,
    });
    run.status = PipelineStatus::Cancelled;
}
