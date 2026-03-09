/// Command-line argument parsing for the MCP server binary.

pub struct Args {
    pub session_id: Option<String>,
}

/// Parses CLI arguments, looking for `--session-id <id>`.
pub fn parse_args() -> Args {
    let args: Vec<String> = std::env::args().collect();
    let mut session_id = None;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--session-id" && i + 1 < args.len() {
            session_id = Some(args[i + 1].clone());
            i += 2;
        } else {
            i += 1;
        }
    }
    Args { session_id }
}
