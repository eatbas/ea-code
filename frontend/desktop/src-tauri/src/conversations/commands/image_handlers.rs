use base64::Engine;
use crate::models::ImageSaveResult;
use super::super::persistence;

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
