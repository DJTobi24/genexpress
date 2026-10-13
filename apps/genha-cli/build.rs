//! Windows: embeds the app icon and version information in `genha-cli.exe` (`genha_brand`).
//! Other systems: nothing (the macOS bundle and the Linux `.desktop` file carry the icon; see
//! `cargo xtask bundle`).

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    windows::embed("GenHa command line");
}

#[cfg(windows)]
mod windows {
    pub fn embed(description: &str) {
        let Some(out) = std::env::var_os("OUT_DIR") else {
            println!("cargo:warning=OUT_DIR is not set: the executable gets no icon");
            return;
        };
        let version = std::env::var("CARGO_PKG_VERSION").unwrap_or_default();
        match genha_brand::windows_resource(std::path::Path::new(&out), description, &version) {
            Ok(rc) => {
                if let Err(e) = embed_resource::compile(&rc, embed_resource::NONE).manifest_optional() {
                    println!("cargo:warning=the executable gets no icon: {e}");
                }
            }
            Err(e) => println!("cargo:warning=the executable gets no icon: {e}"),
        }
    }
}

/// Cross-compiling for Windows from another system: no resource compiler is assumed.
#[cfg(not(windows))]
mod windows {
    pub fn embed(_description: &str) {
        println!("cargo:warning=built on a non-Windows system: the executable gets no icon");
    }
}
