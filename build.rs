fn main() {
    #[cfg(feature = "native-view")]
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(&[
                "preview_html",
                "save_deck",
            ])),
    )
    .expect("tauri build");
}
