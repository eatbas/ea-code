use std::fs;
use crate::models::{ImageEntry, ImageSaveResult};
use crate::storage::with_conversations_lock;
use super::paths::images_dir_path;

const ALLOWED_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "bmp"];

pub fn save_image(
    workspace_path: &str,
    conversation_id: &str,
    image_bytes: &[u8],
    extension: &str,
) -> Result<ImageSaveResult, String> {
    let ext = extension.to_lowercase();
    if !ALLOWED_EXTENSIONS.contains(&ext.as_str()) {
        return Err(format!("Unsupported image extension: {ext}"));
    }

    with_conversations_lock(|| {
        let images_dir = images_dir_path(workspace_path, conversation_id);
        fs::create_dir_all(&images_dir)
            .map_err(|e| format!("Failed to create images directory: {e}"))?;

        let next_index = max_image_index(&images_dir)?;
        let file_name = format!("image{}.{}", next_index + 1, ext);
        let file_path = images_dir.join(&file_name);

        fs::write(&file_path, image_bytes)
            .map_err(|e| format!("Failed to write image file: {e}"))?;

        Ok(ImageSaveResult {
            file_name,
            file_path: file_path.to_string_lossy().into_owned(),
        })
    })
}

/// Lists all image files stored for a conversation, returning their names and
/// absolute paths so the frontend can load them via the asset protocol.
pub fn list_images(
    workspace_path: &str,
    conversation_id: &str,
) -> Result<Vec<ImageEntry>, String> {
    let images_dir = images_dir_path(workspace_path, conversation_id);
    if !images_dir.exists() {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(&images_dir)
        .map_err(|e| format!("Failed to read images directory: {e}"))?;

    let mut images: Vec<ImageEntry> = entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_file())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ALLOWED_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
                .unwrap_or(false)
        })
        .map(|entry| {
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().into_owned();
            let file_path = path.to_string_lossy().into_owned();
            ImageEntry {
                file_name,
                file_path,
            }
        })
        .collect();

    images.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    Ok(images)
}

/// Returns the highest numeric index N found in filenames matching `image{N}.{ext}`,
/// or 0 if the directory is empty or contains no matching files.
/// Using max index + 1 prevents collisions when files are manually deleted.
fn max_image_index(dir: &std::path::Path) -> Result<usize, String> {
    let entries = fs::read_dir(dir)
        .map_err(|e| format!("Failed to read images directory: {e}"))?;

    let max = entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_file())
        .filter_map(|entry| {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            let stem = name_str.split('.').next()?;
            let n_str = stem.strip_prefix("image")?;
            n_str.parse::<usize>().ok()
        })
        .max()
        .unwrap_or(0);

    Ok(max)
}
