#[cfg(not(target_os = "android"))]
#[tauri::command]
async fn generate_image(
    request: ImageGenerationRequest,
    state: State<'_, AppState>,
) -> Result<ImageGenerationResult, String> {
    generate_images(state.inner(), request).await
}
