fn main() {
    // Re-run this build script whenever a translation file changes, so the
    // bundled UI translations stay in sync with the `.po` catalogs.
    println!("cargo:rerun-if-changed=lang");

    // Bundle the gettext `.po` translations into the binary so the UI can switch
    // language at runtime with no gettext C dependency. The translation context
    // is disabled (DefaultTranslationContext::None) so msgids are the bare
    // English text — that lets the very same `.po` files also drive the
    // Rust-side `i18n::t()` lookups (one catalog per language for the whole app).
    let config = slint_build::CompilerConfiguration::new()
        .with_bundled_translations("lang")
        .with_default_translation_context(slint_build::DefaultTranslationContext::None);
    slint_build::compile_with_config("ui/main.slint", config).expect("Slint build failed");

    // Embed the app icon (taskbar / alt-tab / explorer). Best-effort: if no
    // resource compiler is available the build still succeeds, and the tray
    // simply falls back to the generic icon. Regenerate assets/snipdeck.ico
    // with `cargo run --example gen_icon`.
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rerun-if-changed=assets/snipdeck.ico");
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/snipdeck.ico");
        if let Err(e) = res.compile() {
            println!("cargo:warning=app icon embed skipped ({e})");
        }
    }
}
