fn main() {
    #[cfg(feature = "native-view")]
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(&[
                "preview_html",
                "save_deck",
                "render_deck",
                "move_slide",
                "replace_slide",
            ])),
    )
    .expect("tauri build");
}
