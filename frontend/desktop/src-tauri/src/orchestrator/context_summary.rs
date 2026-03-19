use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};

const CONTEXT_CHAR_CAP: usize = 8_000;
const MAX_TREE_LINES: usize = 120;
const MAX_TEST_FILES: usize = 40;

/// Builds a capped workspace context summary for agent system prompts.
pub async fn build_workspace_context_summary(workspace_path: &str) -> String {
    let mut sections = vec![build_workspace_header(workspace_path).await];

    if let Some(section) = build_package_section(workspace_path) {
        sections.push(section);
    }
    if let Some(section) = build_src_tree_section(workspace_path) {
        sections.push(section);
    }
    if let Some(section) = build_test_files_section(workspace_path) {
        sections.push(section);
    }
    // README excluded from workspace context — agents should read it
    // themselves via tools if needed, rather than bloating every prompt.

    truncate_chars(&sections.join("\n\n"), CONTEXT_CHAR_CAP)
}

async fn build_workspace_header(workspace_path: &str) -> String {
    let info = crate::git::workspace_info(workspace_path).await;
    let mut lines = vec![
        "WORKSPACE SNAPSHOT".to_string(),
        format!("Path: {workspace_path}"),
        format!(
            "Git repository: {}",
            if info.is_git_repo { "yes" } else { "no" }
        ),
        format!(
            "Working tree dirty: {}",
            if info.is_dirty { "yes" } else { "no" }
        ),
    ];
    if let Some(branch) = info.branch {
        lines.push(format!("Branch: {branch}"));
    }
    lines.join("\n")
}

fn build_package_section(workspace_path: &str) -> Option<String> {
    let package_json_path = Path::new(workspace_path).join("package.json");
    let raw = fs::read_to_string(package_json_path).ok()?;
    let json = serde_json::from_str::<serde_json::Value>(&raw).ok()?;
    let obj = json.as_object()?;

    let name = obj
        .get("name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("-");
    let version = obj
        .get("version")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("-");
    let scripts = object_keys(obj.get("scripts"), 12);
    let deps = object_keys(obj.get("dependencies"), 16);
    let dev_deps = object_keys(obj.get("devDependencies"), 16);

    Some(format!(
        "PACKAGE.JSON\nname: {name}\nversion: {version}\nscripts: {}\ndependencies: {}\ndevDependencies: {}",
        if scripts.is_empty() { "-".to_string() } else { scripts.join(", ") },
        if deps.is_empty() { "-".to_string() } else { deps.join(", ") },
        if dev_deps.is_empty() { "-".to_string() } else { dev_deps.join(", ") }
    ))
}

fn build_src_tree_section(workspace_path: &str) -> Option<String> {
    let src_root = Path::new(workspace_path).join("src");
    if !src_root.exists() || !src_root.is_dir() {
        return None;
    }

    let mut lines = Vec::new();
    collect_tree_lines(&src_root, &src_root, 0, 2, &mut lines);
    if lines.is_empty() {
        return None;
    }
    if lines.len() > MAX_TREE_LINES {
        lines.truncate(MAX_TREE_LINES);
        lines.push("... (tree truncated)".to_string());
    }

    Some(format!("SRC TREE (depth <= 2)\n{}", lines.join("\n")))
}

fn collect_tree_lines(
    root: &Path,
    current: &Path,
    depth: usize,
    max_depth: usize,
    out: &mut Vec<String>,
) {
    let mut entries = match fs::read_dir(current) {
        Ok(rd) => rd.filter_map(Result::ok).collect::<Vec<_>>(),
        Err(_) => return,
    };
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if should_skip_dir_name(&name) {
            continue;
        }

        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let indent = "  ".repeat(depth);
        if path.is_dir() {
            out.push(format!("{indent}{relative}/"));
            if depth < max_depth {
                collect_tree_lines(root, &path, depth + 1, max_depth, out);
            }
        } else {
            out.push(format!("{indent}{relative}"));
        }
    }
}

fn build_test_files_section(workspace_path: &str) -> Option<String> {
    let root = Path::new(workspace_path);
    let mut queue = VecDeque::from([root.to_path_buf()]);
    let mut hits = Vec::<String>::new();

    while let Some(dir) = queue.pop_front() {
        let entries = match fs::read_dir(&dir) {
            Ok(rd) => rd,
            Err(_) => continue,
        };

        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            if path.is_dir() {
                if should_skip_dir_name(&name) {
                    continue;
                }
                queue.push_back(path);
                continue;
            }

            if looks_like_test_file(&name) {
                let rel = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                hits.push(rel);
                if hits.len() >= MAX_TEST_FILES {
                    break;
                }
            }
        }

        if hits.len() >= MAX_TEST_FILES {
            break;
        }
    }

    if hits.is_empty() {
        return None;
    }
    Some(format!("DISCOVERED TEST FILES\n{}", hits.join("\n")))
}

#[allow(dead_code)]
fn build_readme_section(workspace_path: &str) -> Option<String> {
    let candidates = [PathBuf::from("README.md"), PathBuf::from("readme.md")];
    for candidate in candidates {
        let path = Path::new(workspace_path).join(candidate);
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let snippet = content.lines().take(60).collect::<Vec<_>>().join("\n");
        if !snippet.trim().is_empty() {
            return Some(format!("README HEAD (first 60 lines)\n{snippet}"));
        }
    }
    None
}

fn object_keys(value: Option<&serde_json::Value>, limit: usize) -> Vec<String> {
    let mut keys = value
        .and_then(serde_json::Value::as_object)
        .map(|obj| obj.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    keys.sort();
    if keys.len() > limit {
        keys.truncate(limit);
        keys.push("...".to_string());
    }
    keys
}

fn looks_like_test_file(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.contains(".test.")
        || lower.contains(".spec.")
        || lower.ends_with("_test.rs")
        || lower.starts_with("test_")
}

fn should_skip_dir_name(name: &str) -> bool {
    matches!(
        name,
        ".git" | "node_modules" | "target" | "dist" | "build" | ".next" | ".turbo"
    )
}

fn truncate_chars(text: &str, cap: usize) -> String {
    if text.chars().count() <= cap {
        return text.to_string();
    }
    let clipped = text.chars().take(cap).collect::<String>();
    format!("{clipped}\n\n[context truncated to {cap} chars]")
}
