fn main() {
    #[cfg(feature = "native-view")]
    tauri_build::build();
}
