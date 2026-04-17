use super::super::persistence;
use crate::models::{ImageEntry, ImageSaveResult};
use base64::Engine;

#[tauri::command]
pub async fn save_conversation_image(
    workspace_path: String,
    conversation_id: String,
    image_base64: String,
    extension: String,
) -> Result<ImageSaveResult, String> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&image_base64)
        .map_err(|e| format!("Invalid base64 data: {e}"))?;
    persistence::save_image(&workspace_path, &conversation_id, &bytes, &extension)
}

#[tauri::command]
pub async fn list_conversation_images(
    workspace_path: String,
    conversation_id: String,
) -> Result<Vec<ImageEntry>, String> {
    persistence::list_images(&workspace_path, &conversation_id)
}
