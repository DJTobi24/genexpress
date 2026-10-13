//! `cargo xtask bundle <macos|linux|windows|web>` and `cargo xtask icon`: the downloads of a
//! release (docs/release.md). Everything lands in `dist/`.
//!
//! - **macos**: `GenHa.app` (universal binary from `aarch64-apple-darwin` + `x86_64-apple-darwin`
//!   merged with `lipo`, `Info.plist` with the document types, the icon) and
//!   `GenHa-<version>-macos.dmg`. Signed with `--sign IDENTITY` (Developer ID, hardened runtime,
//!   secure timestamp) or ad hoc; notarized and stapled with `--notary-profile NAME` (a
//!   `xcrun notarytool store-credentials` profile). Runs on macOS only.
//! - **linux**: `GenHa-<version>-linux-<arch>.tar.gz`: `bin/genha`, `bin/genha-cli`, the
//!   `.desktop` file, icons and the MIME type of `.genha` files, laid out like `~/.local`.
//! - **windows**: `GenHa-<version>-windows-<arch>.zip`: `genha.exe` and `genha-cli.exe` (the icon
//!   is embedded by their build scripts).
//! - **web**: `GenHa-<version>-web.zip` of `cargo xtask web`.

use std::path::{Path, PathBuf};
use std::process::Command;

use genha_brand::{APP_ID, APP_NAME, EXTENSION, LEGACY_EXTENSION, MIME_TYPE, Style};

use crate::root;

/// Oldest macOS the app supports (also the deployment target of the build).
pub const MACOS_MIN: &str = "11.0";

/// The binaries every desktop download carries.
const BINARIES: &[&str] = &["genha", "genha-cli"];

/// Sizes of the PNG icons in the Linux download (hicolor theme) and `cargo xtask icon`.
const PNG_SIZES: &[u32] = &[16, 24, 32, 48, 64, 128, 256, 512, 1024];

pub fn main(args: &[String]) -> Result<(), String> {
    let flag = |name: &str| args.iter().any(|a| a == name);
    let value = |name: &str| -> Result<Option<String>, String> {
        match args.iter().position(|a| a == name) {
            Some(i) => args.get(i + 1).filter(|v| !v.starts_with("--")).cloned().map(Some).ok_or_else(|| format!("{name} needs a value")),
            None => Ok(None),
        }
    };
    match args.first().map(String::as_str) {
        Some("macos") => macos(&MacOptions {
            universal: !flag("--native"),
            identity: value("--sign")?.filter(|s| !s.trim().is_empty()),
            notary_profile: value("--notary-profile")?.filter(|s| !s.trim().is_empty()),
            keychain: value("--keychain")?.filter(|s| !s.trim().is_empty()),
            dmg: !flag("--no-dmg"),
        }),
        Some("linux") => linux(),
        Some("windows") => windows(),
        Some("web") => web(flag("--ml")),
        _ => Err("usage: cargo xtask bundle <macos|linux|windows|web> [options] (see `cargo xtask help`)".into()),
    }
}

// ---------------------------------------------------------------------------------------------
// Shared

/// The app's version (the `genha` package in Cargo.toml).
pub fn version() -> Result<String, String> {
    let out = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .current_dir(root())
        .output()
        .map_err(|e| format!("cannot run cargo metadata: {e}"))?;
    let meta: serde_json::Value = serde_json::from_slice(&out.stdout).map_err(|e| format!("cargo metadata: {e}"))?;
    meta.get("packages")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .find(|p| p.get("name").and_then(serde_json::Value::as_str) == Some("genha"))
        .and_then(|p| p.get("version")?.as_str().map(String::from))
        .ok_or_else(|| "cargo metadata: no version for the genha package".into())
}

fn target_dir() -> Result<PathBuf, String> {
    crate::web::target_dir(&root())
}

fn dist() -> PathBuf {
    root().join("dist")
}

/// Runs a program in the repository root with extra environment variables; fails on a non-zero
/// exit.
fn exec(program: &str, args: &[&str], env: &[(&str, &str)]) -> Result<(), String> {
    println!("$ {program} {}", args.join(" "));
    let status =
        Command::new(program).args(args).envs(env.iter().copied()).current_dir(root()).status().map_err(|e| format!("cannot run {program}: {e}"))?;
    if status.success() { Ok(()) } else { Err(format!("`{program} {}` failed ({status})", args.join(" "))) }
}

/// Runs a program and returns whether it succeeded and its standard output (standard error goes
/// to the terminal).
fn capture(program: &str, args: &[&str]) -> Result<(bool, String), String> {
    println!("$ {program} {}", args.join(" "));
    let out = Command::new(program)
        .args(args)
        .current_dir(root())
        .stderr(std::process::Stdio::inherit())
        .output()
        .map_err(|e| format!("cannot run {program}: {e}"))?;
    Ok((out.status.success(), String::from_utf8_lossy(&out.stdout).into_owned()))
}

fn s(p: &Path) -> String {
    p.display().to_string()
}

fn fresh_dir(p: &Path) -> Result<(), String> {
    if p.exists() {
        std::fs::remove_dir_all(p).map_err(|e| format!("cannot remove {}: {e}", p.display()))?;
    }
    std::fs::create_dir_all(p).map_err(|e| format!("cannot create {}: {e}", p.display()))
}

fn write(p: &Path, bytes: impl AsRef<[u8]>) -> Result<(), String> {
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    }
    std::fs::write(p, bytes).map_err(|e| format!("cannot write {}: {e}", p.display()))
}

fn copy(from: &Path, to: &Path) -> Result<(), String> {
    if let Some(dir) = to.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    }
    std::fs::copy(from, to).map(|_| ()).map_err(|e| format!("cannot copy {} to {}: {e}", from.display(), to.display()))
}

#[cfg(unix)]
fn make_executable(p: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o755)).map_err(|e| format!("cannot chmod {}: {e}", p.display()))
}

#[cfg(not(unix))]
fn make_executable(_p: &Path) -> Result<(), String> {
    Ok(())
}

/// `cargo build --release` of the desktop binaries, stripped of symbols (smaller downloads).
fn build_release(target: Option<&str>, env: &[(&str, &str)]) -> Result<(), String> {
    let mut args = vec!["build", "--release", "--locked"];
    for b in BINARIES {
        args.extend(["-p", b]);
    }
    if let Some(t) = target {
        args.extend(["--target", t]);
    }
    let mut env = env.to_vec();
    env.push(("CARGO_PROFILE_RELEASE_STRIP", "symbols"));
    exec("cargo", &args, &env)
}

/// Where `build_release` put binary `name`.
fn built(target: Option<&str>, name: &str) -> Result<PathBuf, String> {
    let mut dir = target_dir()?;
    if let Some(t) = target {
        dir = dir.join(t);
    }
    let file = if cfg!(windows) { format!("{name}.exe") } else { name.to_string() };
    let p = dir.join("release").join(file);
    if p.exists() { Ok(p) } else { Err(format!("{} was not built", p.display())) }
}

/// License files every download carries.
fn copy_licenses(into: &Path) -> Result<(), String> {
    for f in ["LICENSE-MIT", "LICENSE-APACHE", "NOTICE"] {
        copy(&root().join(f), &into.join(f))?;
    }
    Ok(())
}

/// Linux and Windows names of the machine: x86_64 or aarch64.
fn arch() -> &'static str {
    std::env::consts::ARCH
}

// ---------------------------------------------------------------------------------------------
// Icons

/// `cargo xtask icon [--out DIR]`: PNG icons in both styles, `genha.ico` and `GenHa.icns`.
pub fn icon(args: &[String]) -> Result<(), String> {
    let out = match args.iter().position(|a| a == "--out") {
        Some(i) => PathBuf::from(args.get(i + 1).ok_or("--out needs a folder")?),
        None => dist().join("icon"),
    };
    std::fs::create_dir_all(&out).map_err(|e| format!("cannot create {}: {e}", out.display()))?;
    for &size in PNG_SIZES {
        write(&out.join(format!("genha-{size}.png")), genha_brand::png(size, Style::Full).map_err(|e| e.to_string())?)?;
        write(&out.join(format!("genha-macos-{size}.png")), genha_brand::png(size, Style::MacOs).map_err(|e| e.to_string())?)?;
    }
    write(&out.join("genha.ico"), genha_brand::ico().map_err(|e| e.to_string())?)?;
    write(&out.join("GenHa.icns"), genha_brand::icns().map_err(|e| e.to_string())?)?;
    println!("icon: wrote {} PNG files, genha.ico and GenHa.icns to {}", PNG_SIZES.len() * 2, out.display());
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// macOS

pub struct MacOptions {
    pub universal: bool,
    pub identity: Option<String>,
    pub notary_profile: Option<String>,
    pub keychain: Option<String>,
    pub dmg: bool,
}

const MAC_TARGETS: &[&str] = &["aarch64-apple-darwin", "x86_64-apple-darwin"];

fn macos(o: &MacOptions) -> Result<(), String> {
    if !cfg!(target_os = "macos") {
        return Err("`cargo xtask bundle macos` runs on macOS (it needs lipo, iconutil, codesign and hdiutil)".into());
    }
    if o.notary_profile.is_some() && o.identity.is_none() {
        return Err("--notary-profile needs --sign: Apple notarizes only apps signed with a Developer ID".into());
    }
    let version = version()?;
    let work = dist().join("macos");
    std::fs::create_dir_all(&work).map_err(|e| format!("cannot create {}: {e}", work.display()))?;
    let deployment = [("MACOSX_DEPLOYMENT_TARGET", MACOS_MIN)];

    // 1. Binaries: one per architecture, merged into universal ones.
    let bins = work.join("bin");
    fresh_dir(&bins)?;
    if o.universal {
        // Installs the targets when rustup manages the toolchain (a no-op when they are there).
        if Command::new("rustup").arg("--version").output().is_ok_and(|r| r.status.success()) {
            let mut args = vec!["target", "add"];
            args.extend(MAC_TARGETS);
            exec("rustup", &args, &[])?;
        }
        for t in MAC_TARGETS {
            build_release(Some(t), &deployment)?;
        }
        for b in BINARIES {
            let mut args = vec!["-create".to_string(), "-output".to_string(), s(&bins.join(b))];
            for t in MAC_TARGETS {
                args.push(s(&built(Some(t), b)?));
            }
            let args: Vec<&str> = args.iter().map(String::as_str).collect();
            exec("lipo", &args, &[])?;
            exec("lipo", &["-info", &s(&bins.join(b))], &[])?;
        }
    } else {
        build_release(None, &deployment)?;
        for b in BINARIES {
            copy(&built(None, b)?, &bins.join(b))?;
        }
    }

    // 2. The bundle.
    let app = work.join(format!("{APP_NAME}.app"));
    let icns = mac_icns(&work)?;
    write_app_bundle(&app, &bins, &icns, &version)?;

    // 3. Signing (hardened runtime and secure timestamp with a Developer ID; ad hoc otherwise,
    // which Apple silicon requires to run at all).
    let sign = |path: &Path| -> Result<(), String> {
        let p = s(path);
        match &o.identity {
            Some(id) => {
                let mut args = vec!["--force", "--options", "runtime", "--timestamp", "--sign", id.as_str()];
                if let Some(k) = &o.keychain {
                    args.extend(["--keychain", k.as_str()]);
                }
                args.push(&p);
                exec("codesign", &args, &[])
            }
            None => exec("codesign", &["--force", "--sign", "-", &p], &[]),
        }
    };
    // Nested code first, the bundle (and with it the main executable) last.
    sign(&app.join("Contents/MacOS/genha-cli"))?;
    sign(&app)?;
    exec("codesign", &["--verify", "--deep", "--strict", "--verbose=2", &s(&app)], &[])?;

    // 4. Notarization of the app, so the copy people drag to Applications carries its ticket.
    if let Some(profile) = &o.notary_profile {
        let zip = work.join(format!("{APP_NAME}-notarize.zip"));
        exec("ditto", &["-c", "-k", "--keepParent", &s(&app), &s(&zip)], &[])?;
        notarize(&zip, profile, o.keychain.as_deref())?;
        // The zip only carried the app to Apple.
        let _ = std::fs::remove_file(&zip);
        exec("xcrun", &["stapler", "staple", &s(&app)], &[])?;
        exec("xcrun", &["stapler", "validate", &s(&app)], &[])?;
        exec("spctl", &["--assess", "-vv", "--type", "execute", &s(&app)], &[])?;
    }

    // 5. The disk image: the app and a link to /Applications.
    if o.dmg {
        let name = if o.universal { format!("{APP_NAME}-{version}-macos.dmg") } else { format!("{APP_NAME}-{version}-macos-{}.dmg", arch()) };
        let dmg = dist().join(&name);
        let stage = work.join("dmg");
        fresh_dir(&stage)?;
        exec("ditto", &[&s(&app), &s(&stage.join(format!("{APP_NAME}.app")))], &[])?;
        symlink("/Applications", &stage.join("Applications"))?;
        if dmg.exists() {
            std::fs::remove_file(&dmg).map_err(|e| format!("cannot remove {}: {e}", dmg.display()))?;
        }
        // hdiutil now and then fails with "Resource busy" on build machines: try a few times.
        let create = ["create", "-volname", APP_NAME, "-srcfolder", &s(&stage), "-fs", "HFS+", "-format", "UDZO", "-ov", &s(&dmg)];
        let mut attempt = 1;
        while let Err(e) = exec("hdiutil", &create, &[]) {
            if attempt == 3 {
                return Err(e);
            }
            println!("bundle: {e}; trying again");
            std::thread::sleep(std::time::Duration::from_secs(5 * attempt));
            attempt += 1;
        }
        if let Some(id) = &o.identity {
            let mut args = vec!["--force", "--timestamp", "--sign", id.as_str()];
            if let Some(k) = &o.keychain {
                args.extend(["--keychain", k.as_str()]);
            }
            let d = s(&dmg);
            args.push(&d);
            exec("codesign", &args, &[])?;
            exec("codesign", &["--verify", "--verbose=2", &d], &[])?;
        }
        if let Some(profile) = &o.notary_profile {
            notarize(&dmg, profile, o.keychain.as_deref())?;
            exec("xcrun", &["stapler", "staple", &s(&dmg)], &[])?;
            exec("xcrun", &["stapler", "validate", &s(&dmg)], &[])?;
            // Gatekeeper's verdict on the disk image, as on a Mac that downloaded it ("accepted,
            // source=Notarized Developer ID"). Apple documents `-t open` with the primary
            // signature context for disk images; `-t install` is printed as well.
            exec("spctl", &["--assess", "-vv", "--type", "open", "--context", "context:primary-signature", &s(&dmg)], &[])?;
            if exec("spctl", &["--assess", "-vv", "--type", "install", &s(&dmg)], &[]).is_err() {
                println!("bundle: note: `spctl -t install` did not accept the disk image (the check above did)");
            }
        }
        println!("bundle: {}", dmg.display());
    }
    let how = match (&o.identity, &o.notary_profile) {
        (Some(_), Some(_)) => "signed with a Developer ID, notarized and stapled",
        (Some(_), None) => "signed with a Developer ID, NOT notarized (Gatekeeper warns on other Macs)",
        _ => "ad-hoc signed, NOT notarized (other Macs: right-click › Open, see docs/release.md)",
    };
    println!("bundle: {} ({how})", app.display());
    Ok(())
}

#[cfg(unix)]
fn symlink(target: &str, link: &Path) -> Result<(), String> {
    std::os::unix::fs::symlink(target, link).map_err(|e| format!("cannot link {}: {e}", link.display()))
}

#[cfg(not(unix))]
fn symlink(_target: &str, _link: &Path) -> Result<(), String> {
    Err("symbolic links need macOS".into())
}

/// The `.icns` file: `iconutil` on an `.iconset` when it is there (always on macOS), else
/// `genha_brand`'s own writer.
fn mac_icns(work: &Path) -> Result<Vec<u8>, String> {
    let set = work.join(format!("{APP_NAME}.iconset"));
    fresh_dir(&set)?;
    for (name, size) in genha_brand::ICONSET {
        write(&set.join(name), genha_brand::png(*size, Style::MacOs).map_err(|e| e.to_string())?)?;
    }
    let out = work.join(format!("{APP_NAME}.icns"));
    if exec("iconutil", &["-c", "icns", "-o", &s(&out), &s(&set)], &[]).is_ok()
        && let Ok(bytes) = std::fs::read(&out)
    {
        return Ok(bytes);
    }
    println!("bundle: iconutil failed or is missing; writing the .icns file directly");
    genha_brand::icns().map_err(|e| e.to_string())
}

/// Lays out `GenHa.app` at `app` with the binaries in `bins`, the icon and `Info.plist`.
pub fn write_app_bundle(app: &Path, bins: &Path, icns: &[u8], version: &str) -> Result<(), String> {
    fresh_dir(app)?;
    let contents = app.join("Contents");
    for b in BINARIES {
        let to = contents.join("MacOS").join(b);
        copy(&bins.join(b), &to)?;
        make_executable(&to)?;
    }
    write(&contents.join("Resources").join(format!("{APP_NAME}.icns")), icns)?;
    write(&contents.join("Info.plist"), info_plist(version))?;
    write(&contents.join("PkgInfo"), "APPL????")
}

/// `CFBundleVersion` and `CFBundleShortVersionString` take up to three numbers: `0.2.0-beta.1`
/// becomes `0.2.0`.
fn plist_version(version: &str) -> String {
    let core = version.split(['-', '+']).next().unwrap_or(version);
    let nums: Vec<&str> = core.split('.').filter(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit())).take(3).collect();
    if nums.is_empty() { "0.0.0".into() } else { nums.join(".") }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// The bundle's `Info.plist`.
pub fn info_plist(version: &str) -> String {
    let v = plist_version(version);
    let id = xml_escape(APP_ID);
    let uti = xml_escape(&genha_brand::document_uti());
    let legacy_uti = xml_escape(&genha_brand::legacy_document_uti());
    let year_note = "GenHa contributors. MIT OR Apache-2.0.";
    let folder = |place: &str| format!("GenHa opens and saves your designs and pictures in your {place} folder when you choose them.");
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleDevelopmentRegion</key>
	<string>en</string>
	<key>CFBundleDisplayName</key>
	<string>{APP_NAME}</string>
	<key>CFBundleExecutable</key>
	<string>genha</string>
	<key>CFBundleIconFile</key>
	<string>{APP_NAME}</string>
	<key>CFBundleIdentifier</key>
	<string>{id}</string>
	<key>CFBundleInfoDictionaryVersion</key>
	<string>6.0</string>
	<key>CFBundleName</key>
	<string>{APP_NAME}</string>
	<key>CFBundlePackageType</key>
	<string>APPL</string>
	<key>CFBundleShortVersionString</key>
	<string>{v}</string>
	<key>CFBundleVersion</key>
	<string>{v}</string>
	<key>CFBundleSupportedPlatforms</key>
	<array>
		<string>MacOSX</string>
	</array>
	<key>LSMinimumSystemVersion</key>
	<string>{MACOS_MIN}</string>
	<key>LSApplicationCategoryType</key>
	<string>public.app-category.graphics-design</string>
	<key>NSHighResolutionCapable</key>
	<true/>
	<key>NSSupportsAutomaticGraphicsSwitching</key>
	<true/>
	<key>NSHumanReadableCopyright</key>
	<string>{year_note}</string>
	<key>NSDesktopFolderUsageDescription</key>
	<string>{desktop}</string>
	<key>NSDocumentsFolderUsageDescription</key>
	<string>{documents}</string>
	<key>NSDownloadsFolderUsageDescription</key>
	<string>{downloads}</string>
	<key>CFBundleDocumentTypes</key>
	<array>
		<dict>
			<key>CFBundleTypeName</key>
			<string>GenHa Design</string>
			<key>CFBundleTypeRole</key>
			<string>Editor</string>
			<key>LSHandlerRank</key>
			<string>Owner</string>
			<key>CFBundleTypeIconFile</key>
			<string>{APP_NAME}</string>
			<key>LSItemContentTypes</key>
			<array>
				<string>{uti}</string>
			</array>
		</dict>
		<dict>
			<key>CFBundleTypeName</key>
			<string>GenExpress Design</string>
			<key>CFBundleTypeRole</key>
			<string>Editor</string>
			<key>LSHandlerRank</key>
			<string>Owner</string>
			<key>CFBundleTypeIconFile</key>
			<string>{APP_NAME}</string>
			<key>LSItemContentTypes</key>
			<array>
				<string>{legacy_uti}</string>
			</array>
		</dict>
		<dict>
			<key>CFBundleTypeName</key>
			<string>Picture</string>
			<key>CFBundleTypeRole</key>
			<string>Editor</string>
			<key>LSHandlerRank</key>
			<string>Alternate</string>
			<key>LSItemContentTypes</key>
			<array>
				<string>public.png</string>
				<string>public.jpeg</string>
				<string>org.webmproject.webp</string>
				<string>com.compuserve.gif</string>
				<string>com.microsoft.bmp</string>
				<string>public.svg-image</string>
			</array>
		</dict>
	</array>
	<key>UTExportedTypeDeclarations</key>
	<array>
		<dict>
			<key>UTTypeIdentifier</key>
			<string>{uti}</string>
			<key>UTTypeDescription</key>
			<string>GenHa Design</string>
			<key>UTTypeIconFile</key>
			<string>{APP_NAME}</string>
			<key>UTTypeConformsTo</key>
			<array>
				<string>public.data</string>
				<string>public.content</string>
			</array>
			<key>UTTypeTagSpecification</key>
			<dict>
				<key>public.filename-extension</key>
				<array>
					<string>{EXTENSION}</string>
				</array>
				<key>public.mime-type</key>
				<array>
					<string>{MIME_TYPE}</string>
				</array>
			</dict>
		</dict>
		<dict>
			<key>UTTypeIdentifier</key>
			<string>{legacy_uti}</string>
			<key>UTTypeDescription</key>
			<string>GenExpress Design</string>
			<key>UTTypeIconFile</key>
			<string>{APP_NAME}</string>
			<key>UTTypeConformsTo</key>
			<array>
				<string>public.data</string>
				<string>public.content</string>
			</array>
			<key>UTTypeTagSpecification</key>
			<dict>
				<key>public.filename-extension</key>
				<array>
					<string>{LEGACY_EXTENSION}</string>
				</array>
			</dict>
		</dict>
	</array>
</dict>
</plist>
"#,
        desktop = folder("Desktop"),
        documents = folder("Documents"),
        downloads = folder("Downloads"),
    )
}

/// Submits `file` to Apple's notary service and waits; fails with Apple's log unless accepted.
fn notarize(file: &Path, profile: &str, keychain: Option<&str>) -> Result<(), String> {
    let f = s(file);
    let mut args = vec!["notarytool", "submit", f.as_str(), "--keychain-profile", profile, "--wait", "--output-format", "json"];
    if let Some(k) = keychain {
        args.extend(["--keychain", k]);
    }
    // `--wait` returns when Apple has decided (minutes, sometimes longer). The JSON says how, also
    // when the exit status is not zero.
    let (_, out) = capture("xcrun", &args)?;
    let json: serde_json::Value = serde_json::from_str(out.trim())
        .map_err(|e| format!("notarytool did not report a result ({e}): {out}\n(see docs/release.md › Troubleshooting)"))?;
    let status = json.get("status").and_then(serde_json::Value::as_str).unwrap_or("unknown");
    let id = json.get("id").and_then(serde_json::Value::as_str).unwrap_or("");
    println!("notarization of {}: {status} (submission {id})", file.display());
    if status == "Accepted" {
        return Ok(());
    }
    if !id.is_empty() {
        let mut log_args = vec!["notarytool", "log", id, "--keychain-profile", profile];
        if let Some(k) = keychain {
            log_args.extend(["--keychain", k]);
        }
        // The log explains the rejection; the error below is reported either way.
        if let Ok((_, log)) = capture("xcrun", &log_args) {
            println!("{log}");
        }
    }
    Err(format!("Apple did not accept {} for notarization: {status} (see the log above and docs/release.md › Troubleshooting)", file.display()))
}

// ---------------------------------------------------------------------------------------------
// Linux

fn linux() -> Result<(), String> {
    let version = version()?;
    build_release(None, &[])?;
    let name = format!("{APP_NAME}-{version}-linux-{}", arch());
    let stage = dist().join("linux").join(&name);
    let bins: Vec<(&str, PathBuf)> = BINARIES.iter().map(|b| built(None, b).map(|p| (*b, p))).collect::<Result<_, _>>()?;
    write_linux_tree(&stage, &bins, &version)?;
    let tarball = dist().join(format!("{name}.tar.gz"));
    exec("tar", &["-czf", &s(&tarball), "-C", &s(&dist().join("linux")), &name], &[])?;
    println!("bundle: {}", tarball.display());
    Ok(())
}

/// The Linux download's folder: `bin/`, `share/` (like `~/.local`), licences and a README.
pub fn write_linux_tree(stage: &Path, bins: &[(&str, PathBuf)], version: &str) -> Result<(), String> {
    fresh_dir(stage)?;
    for (name, from) in bins {
        let to = stage.join("bin").join(name);
        copy(from, &to)?;
        make_executable(&to)?;
    }
    let share = stage.join("share");
    write(&share.join("applications").join(format!("{APP_ID}.desktop")), desktop_file())?;
    write(&share.join("mime").join("packages").join(format!("{APP_ID}.xml")), mime_xml())?;
    for &size in PNG_SIZES.iter().filter(|s| **s <= 512) {
        let p = share.join("icons/hicolor").join(format!("{size}x{size}")).join("apps").join(format!("{APP_ID}.png"));
        write(&p, genha_brand::png(size, Style::Full).map_err(|e| e.to_string())?)?;
    }
    copy_licenses(stage)?;
    write(&stage.join("README.txt"), linux_readme(version))
}

/// The `.desktop` entry (freedesktop.org Desktop Entry Specification 1.5).
pub fn desktop_file() -> String {
    format!(
        "[Desktop Entry]
Type=Application
Version=1.5
Name={APP_NAME}
GenericName=Design App
Comment=Design social posts, flyers, posters, presentations and more
Exec=genha %F
Icon={APP_ID}
Terminal=false
Categories=Graphics;2DGraphics;VectorGraphics;RasterGraphics;
MimeType={MIME_TYPE};image/png;image/jpeg;image/webp;image/gif;image/bmp;image/svg+xml;
Keywords=design;poster;flyer;social;template;presentation;
StartupWMClass={APP_ID}
"
    )
}

/// The MIME type of `.genha` files (shared-mime-info).
pub fn mime_xml() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<mime-info xmlns="http://www.freedesktop.org/standards/shared-mime-info">
  <mime-type type="{MIME_TYPE}">
    <comment>GenHa design</comment>
    <icon name="{APP_ID}"/>
    <glob pattern="*.{EXTENSION}"/>
    <glob pattern="*.{LEGACY_EXTENSION}"/>
  </mime-type>
</mime-info>
"#
    )
}

fn linux_readme(version: &str) -> String {
    format!(
        "GenHa {version} for Linux
=====================

Run it from this folder:

    ./bin/genha

Install it for your user (menu entry, icon, .genha files open in GenHa):

    mkdir -p ~/.local
    cp -r bin share ~/.local/
    update-desktop-database ~/.local/share/applications 2>/dev/null || true
    update-mime-database ~/.local/share/mime 2>/dev/null || true

~/.local/bin must be on your PATH (it is on most distributions). Uninstall by deleting
~/.local/bin/genha, ~/.local/bin/genha-cli and the files named {APP_ID}.* under ~/.local/share.

genha-cli is the command line (export, scripts, quick actions, the MCP server): genha-cli --help.

GenHa is open source (MIT OR Apache-2.0): https://github.com/DJTobi24/genexpress
"
    )
}

// ---------------------------------------------------------------------------------------------
// Windows

fn windows() -> Result<(), String> {
    if !cfg!(windows) {
        return Err("`cargo xtask bundle windows` runs on Windows (the icon is embedded with the Windows resource compiler)".into());
    }
    let version = version()?;
    build_release(None, &[])?;
    let name = format!("{APP_NAME}-{version}-windows-{}", arch());
    let stage = dist().join("windows").join(&name);
    fresh_dir(&stage)?;
    for b in BINARIES {
        copy(&built(None, b)?, &stage.join(format!("{b}.exe")))?;
    }
    copy_licenses(&stage)?;
    write(&stage.join("README.txt"), windows_readme(&version))?;
    let zip = dist().join(format!("{name}.zip"));
    crate::zip::zip_dir(&stage, &name, &zip)?;
    println!("bundle: {}", zip.display());
    Ok(())
}

fn windows_readme(version: &str) -> String {
    format!(
        "GenHa {version} for Windows\r
=======================\r
\r
Start genha.exe. No installation is needed: keep this folder where you like (for example in\r
Documents or Program Files) and pin genha.exe to Start or the taskbar.\r
\r
These programs are not code-signed yet, so Windows SmartScreen may say \"Windows protected your\r
PC\" the first time: choose \"More info\", then \"Run anyway\".\r
\r
genha-cli.exe is the command line (export, scripts, quick actions, the MCP server):\r
genha-cli --help.\r
\r
GenHa is open source (MIT OR Apache-2.0): https://github.com/DJTobi24/genexpress\r
"
    )
}

// ---------------------------------------------------------------------------------------------
// Web

fn web(ml: bool) -> Result<(), String> {
    let version = version()?;
    crate::web::build(&root(), ml)?;
    let name = format!("{APP_NAME}-{version}-web");
    let zip = dist().join(format!("{name}.zip"));
    crate::zip::zip_dir(&dist().join("web"), &name, &zip)?;
    println!("bundle: {}", zip.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("genha-xtask-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn plist_versions_are_numbers() {
        assert_eq!(plist_version("0.1.0"), "0.1.0");
        assert_eq!(plist_version("1.2.3-beta.4"), "1.2.3");
        assert_eq!(plist_version("2.0.0+build.7"), "2.0.0");
        assert_eq!(plist_version("junk"), "0.0.0");
    }

    #[test]
    fn info_plist_declares_the_app_and_its_documents() {
        let p = info_plist("0.1.0");
        for needle in [
            "<key>CFBundleIdentifier</key>\n\t<string>eu.tobicloud.genha</string>",
            "<key>CFBundleShortVersionString</key>\n\t<string>0.1.0</string>",
            "<key>CFBundleVersion</key>\n\t<string>0.1.0</string>",
            "<key>LSMinimumSystemVersion</key>\n\t<string>11.0</string>",
            "<key>NSHighResolutionCapable</key>\n\t<true/>",
            "public.app-category.graphics-design",
            "<string>genha</string>",
            "<string>genexpress</string>",
            "<string>eu.tobicloud.genha.document</string>",
        ] {
            assert!(p.contains(needle), "missing {needle:?}");
        }
        // Balanced XML elements.
        for tag in ["dict", "array", "plist"] {
            assert_eq!(
                p.matches(&format!("<{tag}>")).count() + p.matches(&format!("<{tag} ")).count(),
                p.matches(&format!("</{tag}>")).count(),
                "{tag}"
            );
        }
        // Validate with plutil when it is there (macOS).
        let dir = scratch("plist");
        let f = dir.join("Info.plist");
        std::fs::write(&f, &p).unwrap();
        if let Ok(out) = Command::new("plutil").arg("-lint").arg(&f).output() {
            assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stdout));
        }
        if let Ok(out) = Command::new("xmllint").arg("--noout").arg(&f).output() {
            assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        }
    }

    #[test]
    fn app_bundle_layout() {
        let dir = scratch("app");
        let bins = dir.join("bin");
        std::fs::create_dir_all(&bins).unwrap();
        for b in BINARIES {
            std::fs::write(bins.join(b), b"#!/bin/sh\n").unwrap();
        }
        let app = dir.join("GenHa.app");
        write_app_bundle(&app, &bins, b"icns", "0.1.0").unwrap();
        for f in ["Contents/Info.plist", "Contents/PkgInfo", "Contents/MacOS/genha", "Contents/MacOS/genha-cli", "Contents/Resources/GenHa.icns"] {
            assert!(app.join(f).is_file(), "{f}");
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(app.join("Contents/MacOS/genha")).unwrap().permissions().mode() & 0o111, 0o111);
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn linux_tree_layout() {
        let dir = scratch("linux");
        let bin = dir.join("genha-bin");
        std::fs::write(&bin, b"\x7fELF").unwrap();
        let stage = dir.join("GenHa-0.1.0-linux-x86_64");
        write_linux_tree(&stage, &[("genha", bin.clone()), ("genha-cli", bin)], "0.1.0").unwrap();
        for f in [
            "bin/genha",
            "bin/genha-cli",
            "share/applications/eu.tobicloud.genha.desktop",
            "share/mime/packages/eu.tobicloud.genha.xml",
            "share/icons/hicolor/48x48/apps/eu.tobicloud.genha.png",
            "share/icons/hicolor/512x512/apps/eu.tobicloud.genha.png",
            "LICENSE-MIT",
            "LICENSE-APACHE",
            "README.txt",
        ] {
            assert!(stage.join(f).is_file(), "{f}");
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn desktop_entry_is_well_formed() {
        let d = desktop_file();
        assert!(d.starts_with("[Desktop Entry]\n"));
        for line in d.lines().skip(1) {
            assert!(line.contains('='), "{line}");
        }
        assert!(d.contains("Exec=genha %F\n"));
        assert!(d.contains(&format!("Icon={APP_ID}\n")));
        assert!(d.contains("MimeType=application/x-genha;"));
        if let Ok(out) = Command::new("desktop-file-validate").arg("--help").output()
            && out.status.success()
        {
            let dir = scratch("desktop");
            let f = dir.join(format!("{APP_ID}.desktop"));
            std::fs::write(&f, &d).unwrap();
            let out = Command::new("desktop-file-validate").arg(&f).output().unwrap();
            assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stdout));
        }
    }
}
