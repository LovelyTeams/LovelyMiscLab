use crate::error::AppError;
use misclab_core::artifacts::{repository, ArtifactMetadata};
use uuid::Uuid;

fn parse_id(id: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(id).map_err(|_| AppError::new("artifact", "无效的产物 UUID"))
}

#[tauri::command]
pub fn artifact_metadata(id: String) -> Result<ArtifactMetadata, AppError> {
    Ok(repository().metadata(parse_id(&id)?)?)
}

#[tauri::command]
pub async fn artifact_read(id: String, offset: u64, length: usize) -> Result<Vec<u8>, AppError> {
    let id = parse_id(&id)?;
    tauri::async_runtime::spawn_blocking(move || repository().read(id, offset, length))
        .await
        .map_err(|e| AppError::new("join", e.to_string()))?
        .map_err(Into::into)
}

#[tauri::command]
pub async fn artifact_put(
    name: String,
    mime: String,
    bytes: Vec<u8>,
) -> Result<ArtifactMetadata, AppError> {
    tauri::async_runtime::spawn_blocking(move || repository().put(&name, &mime, &bytes))
        .await
        .map_err(|e| AppError::new("join", e.to_string()))?
        .map_err(Into::into)
}

#[tauri::command]
pub async fn artifact_export(id: String, path: String) -> Result<(), AppError> {
    let id = parse_id(&id)?;
    tauri::async_runtime::spawn_blocking(move || {
        repository().export(id, std::path::Path::new(&path))
    })
    .await
    .map_err(|e| AppError::new("join", e.to_string()))?
    .map_err(Into::into)
}
