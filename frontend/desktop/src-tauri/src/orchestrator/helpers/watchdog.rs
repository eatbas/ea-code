//! Per-agent file watchdog for text stages (planners / reviewers).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Per-agent file watchdog for text stages (planners / reviewers).
///
/// Each agent gets its own `abort_flag`. The watchdog scans `iter_dir` for
/// any `.md` file whose name starts with `slot_prefix` (e.g. `"plan_1"` for
/// planner 1, `"plan_2"` for planner 2). The slot prefix is derived from the
/// agent's `output_kind` -- e.g. `plan_1_claude_opus-4` -> slot prefix `plan_1`.
///
/// As soon as a matching file appears and is stable (unchanged for 5 seconds),
/// the watchdog sets the abort_flag for **that specific agent**, causing its
/// SSE reader to cancel the lingering CLI process.
///
/// This means if kimi writes `plan_1.md` at 30s but claude is still working
/// on `plan_2.md`, only kimi is aborted -- claude keeps running normally.
pub async fn per_agent_file_watchdog(
    iter_dir: String,
    slot_prefix: String,
    abort_flag: Arc<AtomicBool>,
) {
    use tokio::time::{sleep, Duration};

    // Poll every 5 seconds for up to 15 minutes.
    for _ in 0..180 {
        sleep(Duration::from_secs(5)).await;
        if abort_flag.load(Ordering::Relaxed) {
            return; // Agent already finished normally or was aborted.
        }

        if let Ok(entries) = std::fs::read_dir(&iter_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if !name_str.ends_with(".md") {
                    continue;
                }
                // Match files starting with the slot prefix (e.g. "plan_1")
                // This catches: plan_1.md, plan_1_kimi.md, plan_1_whatever.md
                if !name_str.starts_with(&*slot_prefix) {
                    continue;
                }
                if let Ok(meta) = entry.metadata() {
                    if meta.len() > 0 {
                        // File found -- wait for stability.
                        let initial_len = meta.len();
                        sleep(Duration::from_secs(5)).await;
                        if let Ok(meta2) = std::fs::metadata(entry.path()) {
                            if meta2.len() == initial_len {
                                eprintln!(
                                    "[info] File watchdog: '{}' is stable ({} bytes), aborting agent (slot '{}')",
                                    name_str, initial_len, slot_prefix,
                                );
                                abort_flag.store(true, Ordering::Relaxed);
                                return;
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Extracts the slot prefix from an output_kind: `plan_1_claude_opus-4` -> `plan_1`.
/// Used by the per-agent watchdog to match files for a specific agent slot.
pub fn slot_prefix_from_output_kind(output_kind: &str) -> String {
    // output_kind format: "{prefix}_{N}_{backend}_{model}" or "{prefix}_{N}"
    // We want "{prefix}_{N}" -- everything up to and including the first digit group.
    let mut end = 0;
    let mut found_digit = false;
    for (i, c) in output_kind.char_indices() {
        if c.is_ascii_digit() {
            found_digit = true;
            end = i + c.len_utf8();
        } else if found_digit {
            // First non-digit after digits -- stop here.
            break;
        }
    }
    if found_digit {
        output_kind[..end].to_string()
    } else {
        output_kind.to_string()
    }
}
