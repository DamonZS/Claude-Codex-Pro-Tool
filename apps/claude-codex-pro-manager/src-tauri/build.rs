fn main() {
    println!("cargo:rerun-if-changed=icons/icon.ico");
    println!("cargo:rerun-if-changed=icons/icon.png");

    // A release exe embeds `../dist`. If the frontend build failed, the dir can
    // exist but be empty, and the app then only shows "asset not found:
    // index.html". Fail the release build instead of shipping that.
    println!("cargo:rerun-if-changed=../dist/index.html");
    if std::env::var("PROFILE").as_deref() == Ok("release")
        && !std::path::Path::new("../dist/index.html").is_file()
    {
        panic!(
            "../dist/index.html is missing: run `npm run vite:build` in apps/claude-codex-pro-manager before `cargo build --release`"
        );
    }

    let windows = tauri_build::WindowsAttributes::new()
        .app_manifest(include_str!("windows-app-manifest.xml"));
    let attrs = tauri_build::Attributes::new().windows_attributes(windows);
    tauri_build::try_build(attrs).expect("failed to run Tauri build script");
}
