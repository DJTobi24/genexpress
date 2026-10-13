# Releases: installable downloads

A release is built by GitHub Actions ([`.github/workflows/release.yml`](../.github/workflows/release.yml))
and published on the repository's **Releases** page:

| Download | What it is |
|---|---|
| `GenHa-<version>-macos.dmg` | `GenHa.app` for macOS 11 or later, one universal binary for Apple silicon and Intel. Signed with a Developer ID, notarized by Apple and stapled when the Apple secrets below are set; otherwise ad-hoc signed (and the job summary says **UNSIGNED**). |
| `GenHa-<version>-windows-x86_64.zip` | `genha.exe` and `genha-cli.exe` with the GenHa icon. Not code-signed: SmartScreen asks once (More info › Run anyway). |
| `GenHa-<version>-linux-x86_64.tar.gz` | `bin/genha`, `bin/genha-cli`, a `.desktop` entry, icons and the MIME type of `.genha` files, laid out like `~/.local` (glibc 2.35 or newer). |
| `GenHa-<version>-web.zip` | The web app (`cargo xtask web`): a static folder to serve over HTTP. |
| `SHA256SUMS.txt` | Checksums of the files above. |

The same packaging runs locally: `cargo xtask bundle macos|linux|windows|web` (each on its own
system; the web one anywhere) and `cargo xtask icon`. Everything lands in `dist/` (ignored by git).

The app id is `eu.tobicloud.genha`: the macOS bundle identifier, the Linux `.desktop` name and the
Wayland app id. It is one constant, `genha_brand::APP_ID` in `crates/brand/src/lib.rs`; the
exported file type of `.genha` designs is `eu.tobicloud.genha.document`.

## The app icon

The icon is original artwork drawn in code (`crates/brand`): a rounded tile in GenHa's accent
violets with a warm circle and a white rounded square in front of it. Every size is drawn from the
shapes (signed distances), so 16 px stays crisp. `cargo xtask icon` writes PNG files from 16 to
1024 px (an edge-to-edge style for Windows and Linux, and Apple's icon grid with a soft shadow for
macOS), `genha.ico` and `GenHa.icns` to `dist/icon`. The desktop app sets it as its window icon;
the Windows build scripts embed it in the executables; `cargo xtask bundle` puts it in the macOS
bundle and the Linux package. No image file is committed except `docs/images/icon.png` (for the
README).

## One-time setup for signed and notarized Mac downloads

You need a paid Apple Developer Program membership. Nothing secret ever goes into the repository:
the values below are entered only in GitHub's secret settings.

### 1. Create a "Developer ID Application" certificate

Either in **Xcode**: Xcode › Settings › Accounts › select your Apple ID › Manage Certificates ›
`+` › **Developer ID Application**.

Or on **developer.apple.com**: Certificates, Identifiers & Profiles › Certificates › `+` ›
**Developer ID Application** (choose the G2 Sub-CA when asked). It asks for a certificate signing
request: on your Mac open Keychain Access › Keychain Access menu › Certificate Assistant ›
Request a Certificate From a Certificate Authority…, enter your e-mail, choose "Saved to disk".
Upload the `.certSigningRequest`, download the `.cer` and double-click it so it lands in your
login keychain next to its private key.

Your signing identity is the certificate's full name, for example
`Developer ID Application: Tobias Example (AB12CD34EF)`. List it with:

```sh
security find-identity -v -p codesigning
```

The ten characters in brackets are your **Team ID**.

### 2. Export it as a .p12 file and base64 it

In Keychain Access › login › My Certificates, right-click the "Developer ID Application: …"
certificate (expand it: the private key must be under it) › Export… › format "Personal
Information Exchange (.p12)" › choose a strong password. Then:

```sh
base64 -i DeveloperID.p12 | pbcopy      # the base64 text is now on the clipboard
```

### 3. Credentials for notarization (one of the two)

**A. App Store Connect API key (recommended).** appstoreconnect.apple.com › Users and Access ›
Integrations › App Store Connect API › Team Keys › `+` › name "GenHa notarization", access
**Developer** › Generate. Note the **Key ID** and the **Issuer ID** (shown above the list), and
download `AuthKey_<KEYID>.p8` (only possible once). Then:

```sh
base64 -i AuthKey_AB12CD34EF.p8 | pbcopy
```

**B. Apple ID with an app-specific password.** account.apple.com › Sign-In and Security ›
App-Specific Passwords › `+` › "GenHa notarization". Use it with your Apple ID e-mail and your
Team ID.

When both are set, the workflow uses the API key.

### 4. Add the GitHub secrets

Repository › **Settings › Secrets and variables › Actions › New repository secret**:

| Secret | Contents |
|---|---|
| `APPLE_CERTIFICATE_P12_BASE64` | the base64 text of the `.p12` file (step 2) |
| `APPLE_CERTIFICATE_PASSWORD` | the password you chose when exporting the `.p12` |
| `APPLE_SIGNING_IDENTITY` | the identity's full name, e.g. `Developer ID Application: Tobias Example (AB12CD34EF)` |
| `APPLE_API_KEY_ID` | (A) the API key's Key ID, e.g. `AB12CD34EF` |
| `APPLE_API_ISSUER_ID` | (A) the Issuer ID, a UUID like `69a6de7e-…` |
| `APPLE_API_KEY_P8_BASE64` | (A) the base64 text of `AuthKey_<KEYID>.p8` (the plain file text, starting with `-----BEGIN PRIVATE KEY-----`, works too) |
| `APPLE_ID` | (B) your Apple ID e-mail |
| `APPLE_TEAM_ID` | (B) your Team ID, e.g. `AB12CD34EF` |
| `APPLE_APP_PASSWORD` | (B) the app-specific password |

Without the first three the macOS job still succeeds, with an ad-hoc signed DMG marked
**UNSIGNED** in the job summary; with them but without notarization credentials, the DMG is signed
but not notarized.

## Cutting a release

1. Set the version in the root `Cargo.toml` (`[workspace.package] version`), commit and push.
2. Tag that commit with the same version and push the tag:

   ```sh
   git tag v0.1.0
   git push origin v0.1.0
   ```

3. The **release** workflow builds the four downloads (the macOS job takes the longest: two
   architectures, then notarization, usually a few minutes per submission), then creates the
   GitHub Release `v0.1.0` with the files, checksums and notes generated from the merged pull
   requests. A tag that does not match the version in `Cargo.toml` stops the run.

To try it without publishing, run it by hand: Actions › **release** › Run workflow. That makes a
**draft** release for the current version (nothing is public until you press Publish on the
Releases page; delete the draft to discard it). Running it again replaces the files of an existing
release with the same tag.

Each job's summary (the run's page) says how the macOS DMG was signed and notarized.

## Checking the downloaded DMG on your Mac

Download the DMG with the browser (so it carries the quarantine flag like everybody else's), then:

```sh
spctl -a -vv -t install ~/Downloads/GenHa-0.1.0-macos.dmg
# … accepted  source=Notarized Developer ID
xcrun stapler validate ~/Downloads/GenHa-0.1.0-macos.dmg
# The validate action worked!
```

Open it, drag GenHa to Applications, then:

```sh
codesign --verify --deep --strict --verbose=2 /Applications/GenHa.app
# /Applications/GenHa.app: valid on disk, satisfies its Designated Requirement
spctl -a -vv -t exec /Applications/GenHa.app
# /Applications/GenHa.app: accepted  source=Notarized Developer ID  origin=Developer ID Application: …
codesign -dv --verbose=2 /Applications/GenHa.app 2>&1 | grep -E 'Authority|TeamIdentifier|Runtime'
lipo -info /Applications/GenHa.app/Contents/MacOS/genha   # x86_64 arm64
```

Start it from Launchpad: no warning should appear. Double-click a `.genha` file: it opens in
GenHa (Finder knows the type from the app's `Info.plist`; the app receives the file through its
application delegate, `apps/genha/src/macos.rs`). The command line is inside the bundle:
`/Applications/GenHa.app/Contents/MacOS/genha-cli --help` (link it into your PATH if you like:
`ln -s /Applications/GenHa.app/Contents/MacOS/genha-cli /usr/local/bin/genha-cli`).

An **unsigned** (ad-hoc) DMG works too, on your own Mac: the first time, right-click GenHa ›
Open › Open (or System Settings › Privacy & Security › Open Anyway).

## Building and signing on your own Mac

Without any certificate (ad-hoc signed, for yourself):

```sh
cargo xtask bundle macos            # universal: dist/macos/GenHa.app and dist/GenHa-<version>-macos.dmg
cargo xtask bundle macos --native   # only this Mac's architecture (half the build time)
open dist/macos/GenHa.app
```

It installs the two Rust targets with rustup when needed (`rustup target add aarch64-apple-darwin
x86_64-apple-darwin`) and needs the Xcode command line tools (`xcode-select --install`) for
`lipo`, `codesign`, `iconutil` and `hdiutil`.

Signed with your Developer ID (the certificate in your login keychain, step 1):

```sh
cargo xtask bundle macos --sign "Developer ID Application: Tobias Example (AB12CD34EF)"
```

Notarized as well: store the credentials once as a notarytool profile in your keychain (either
form), then name it:

```sh
xcrun notarytool store-credentials genha-notary \
  --key ~/keys/AuthKey_AB12CD34EF.p8 --key-id AB12CD34EF --issuer 69a6de7e-0000-0000-0000-000000000000
# or: xcrun notarytool store-credentials genha-notary --apple-id you@example.com --team-id AB12CD34EF
#     (it asks for the app-specific password)

cargo xtask bundle macos --sign "Developer ID Application: Tobias Example (AB12CD34EF)" \
  --notary-profile genha-notary
```

What `cargo xtask bundle macos` does, in order:

1. `cargo build --release --locked -p genha -p genha-cli --target aarch64-apple-darwin` and
   `--target x86_64-apple-darwin` (with `MACOSX_DEPLOYMENT_TARGET=11.0`, symbols stripped), then
   `lipo -create` for each binary;
2. `GenHa.app`: `Contents/MacOS/genha` (the app) and `genha-cli`, `Contents/Resources/GenHa.icns`
   (`iconutil` on a generated `.iconset`), `Contents/Info.plist` (name, `eu.tobicloud.genha`,
   version from `Cargo.toml`, macOS 11.0, high resolution, category graphics & design, the
   document types `.genha` / `.genexpress` with an exported type each, pictures as an alternate
   "Open With" type, and the folder usage texts macOS shows when asking for access);
3. `codesign --force --options runtime --timestamp --sign <identity>` on `genha-cli`, then on the
   app (ad hoc `--sign -` without an identity); `codesign --verify --deep --strict`. The app needs
   no entitlements: it is not sandboxed, and outgoing network (the optional model download) needs
   none outside the sandbox;
4. with `--notary-profile`: zip the app (`ditto`), `xcrun notarytool submit --wait`, `xcrun
   stapler staple` the app, `spctl -a -t exec`;
5. the DMG: the app and a link to `/Applications` (`hdiutil create -format UDZO`), signed with the
   identity; with `--notary-profile` notarized and stapled too, then checked with `spctl`.

In CI the macOS job then smoke-tests the finished app: it starts `GenHa.app` with `open` (Launch
Services, as Finder does) and hands it a design made with the bundled `genha-cli`, then checks
over the control channel that the design opened (`application:openURLs:`) and that the app is
still running ten seconds later. A build that crashes on start fails the job instead of being
published.

The app keeps its data in `~/Library/Application Support/GenHa` (from `HOME`, never from the
working directory), so it behaves the same started from Finder, the Dock or a terminal.

## Troubleshooting

- **"The signature of the binary is invalid" / "not signed with a valid Developer ID
  certificate"** (notarization log): the identity is not a *Developer ID Application* certificate
  (an "Apple Development" or "Mac App Distribution" one does not work), or the `.p12` lacks the
  private key. Export again from My Certificates with the key under the certificate.
- **"The executable does not have the hardened runtime enabled"** or **"The signature does not
  include a secure timestamp"**: something was signed without `--options runtime --timestamp`.
  Use `cargo xtask bundle macos --sign …`, which signs every executable with both.
- **Read Apple's verdict**: `cargo xtask bundle` prints the log of a rejected submission. By hand:
  `xcrun notarytool history --keychain-profile genha-notary`, then
  `xcrun notarytool log <submission-id> --keychain-profile genha-notary` (JSON listing every issue).
- **HTTP 401 / "Unable to authenticate"**: wrong Key ID, Issuer ID or `.p8` (API key), or wrong
  Apple ID, Team ID or app-specific password; an API key needs the Developer role or higher.
  Revoked keys and passwords stop working immediately.
- **"A required agreement is missing or has expired"**: accept the latest agreement in your
  developer account (developer.apple.com › Account), then run the workflow again.
- **CI: "no signing identity named …"**: `APPLE_SIGNING_IDENTITY` must match the name printed by
  `security find-identity -v -p codesigning` exactly (including the Team ID in brackets).
- **CI: `security import` fails ("MAC verification failed")**: `APPLE_CERTIFICATE_PASSWORD` does
  not match the `.p12`, or the base64 text was cut (copy it again with `base64 -i … | pbcopy`).
- **"errSecInternalComponent"** while signing: the keychain is locked or does not allow codesign
  to use the key; locally, unlock the login keychain (`security unlock-keychain`) and allow access
  when asked.
- **Gatekeeper still warns after notarization**: check that the DMG is stapled (`xcrun stapler
  validate`), and that you are testing the downloaded file; `spctl -a -vv -t exec
  /Applications/GenHa.app` names the reason.
- **The app quits right after starting** (a crash report names `abort()` under
  `-[NSApplication _handleEvent:]`): run it from Terminal to see the message,
  `RUST_BACKTRACE=1 /Applications/GenHa.app/Contents/MacOS/genha`. 0.1.0 dry-run builds did this:
  GenHa replaced winit's application delegate, and winit stops the app when it finds another one;
  GenHa now adds its Finder method to winit's delegate instead (`apps/genha/src/macos.rs`).
- **Windows SmartScreen** warns about every unsigned program: More info › Run anyway. Signing the
  Windows build needs a code-signing certificate (not set up yet).
- **Linux: no menu entry after installing**: `~/.local/bin` must be on `PATH`, and the
  desktop/MIME databases need refreshing (`update-desktop-database ~/.local/share/applications`,
  `update-mime-database ~/.local/share/mime`; see README.txt in the package).

## What is not tested automatically

The macOS and Windows jobs can only run on GitHub's macOS and Windows runners: the first tagged
release (or a manual run) is their test. The packaging logic they share with local builds is
unit-tested (`cargo test -p xtask`: `Info.plist`, bundle and Linux layout, zip writer, `.desktop`
file), and the icon writers are tested in `cargo test -p genha-brand` (PNG, `.ico`, `.icns`).
