# lume

**A fast, lightweight home screen for Android TV.** lume replaces the launcher that came with your TV with a clean, remote-friendly one that uses very little memory, so your streaming apps get more of it.

English | [Português](README.pt-BR.md)

> **Status:** lume is in development. Version 0.1.0 has not been released yet, so there is no download yet. The steps below are what you will do once it is out.

<p align="center">
  <img src="docs/screenshots/home-en.png" alt="Home screen with the dock" width="49%">
  <img src="docs/screenshots/apps-en.png" alt="Apps screen" width="49%">
</p>
<p align="center">
  <img src="docs/screenshots/settings-en.png" alt="Settings" width="49%">
  <img src="docs/screenshots/editor-en.png" alt="Editing the favorites" width="49%">
</p>

**What you get**

- A home screen with a floating dock for your favorite apps: the icon under the focus grows, and the neighbors make room.
- An "Apps" screen with everything installed on your TV.
- Everything works with the normal TV remote (arrows, OK, Back, Home).
- Portuguese (Brazil) and English, following your TV's language.
- No internet access at all: lume never connects to the network and asks for no dangerous permissions.

**What you need**

- A TV running **Android TV 9, 10 or 11**.
- A computer (Windows, macOS or Linux) on the **same Wi-Fi or network** as the TV.
- About 10 minutes.

---

## Install lume on your TV

You will send the app to the TV from your computer with a small tool called **adb**. You only do this once.

### 1. Download lume

1. Open the **Releases** page of this project on GitHub.
2. Under the newest version, download the file named like **`lume-0.1.0.apk`**.
3. Remember where you saved it (for example, your Downloads folder).

### 2. Get adb on your computer

1. Download Google's **SDK Platform-Tools** for your system from <https://developer.android.com/tools/releases/platform-tools>.
2. Unzip it somewhere easy, for example `C:\platform-tools` on Windows or `~/platform-tools` on macOS/Linux.
3. Open a terminal in that folder:
   - **Windows:** open the folder, click the address bar, type `cmd` and press Enter.
   - **macOS:** open *Terminal*, type `cd ~/platform-tools` and press Enter.
   - **Linux:** open a terminal and `cd ~/platform-tools`.

To check that it works, type `adb version` (on Windows) or `./adb version` (on macOS/Linux). You should see a version number.

> On macOS and Linux put `./` in front of every `adb` command below (for example `./adb devices`).

### 3. Turn on debugging on the TV

1. On the TV, open **Settings → Device Preferences → About**.
2. Select **Build** (or *Android TV OS build*) and press OK **7 times** until it says you are a developer.
3. Go back to **Device Preferences → Developer options**.
4. Turn on **USB debugging**, and also **Network debugging** / **ADB debugging over network** if you see it.

The exact menu names change a little between TV brands.

### 4. Find the TV's address

On the TV open **Settings → Network & Internet**, select your connection and write down the **IP address**, something like `192.168.1.50`.
(Some TVs also show it under *Developer options* next to the network debugging switch.)

### 5. Connect and install

In the terminal you opened in step 2, type (replace the address with yours):

```
adb connect 192.168.1.50:5555
```

A message will pop up on the TV asking *"Allow USB debugging?"*. Choose **Always allow** and **OK**. Then run the same command again, and check that the TV is listed:

```
adb devices
```

You should see your TV with the word `device` next to it. Now install lume (use the real path of the file you downloaded):

```
adb install "C:\Users\You\Downloads\lume-0.1.0.apk"
```

When you see `Success`, lume is installed.

### 6. Make lume your home screen

1. Press the **Home** button on your remote. Android may ask which home app to use: choose **lume** and **Always**.
2. If nothing was asked, or Home still opens the old launcher, run:

   ```
   adb shell cmd package set-home-activity app.lume.tv/.LumeActivity
   ```

3. Some TVs ignore that and keep opening their own launcher. As a last resort you can turn the original launcher off (this does not delete it and you can turn it back on):

   ```
   adb shell pm disable-user --user 0 com.google.android.tvlauncher
   ```

### Go back to the original launcher

Remove lume, and if you turned the original launcher off, turn it on again:

```
adb shell pm enable com.google.android.tvlauncher
adb uninstall app.lume.tv
```

### Something went wrong?

| Problem | What to try |
|---|---|
| `adb: command not found` / `'adb' is not recognized` | You are not in the platform-tools folder. Open the terminal inside it (step 2). |
| `failed to connect` / `unable to connect` | Check the TV address, that the TV and the computer are on the same network, and that debugging is on. Try turning the TV off and on. |
| `device unauthorized` | Look at the TV screen and accept the prompt, then run `adb devices` again. |
| `INSTALL_FAILED_OLDER_SDK` | Your TV runs Android older than 9. lume cannot run on it. |
| `INSTALL_FAILED_UPDATE_INCOMPATIBLE` | A different build of lume is installed. Run `adb uninstall app.lume.tv` and install again. |
| Home opens the old launcher | Repeat step 6, options 2 and 3. |

---

## For developers

lume is written in Rust. It draws its whole interface itself with OpenGL ES 2.0 (no Views, no WebView, no AndroidX, no Kotlin) and releases all GPU memory while another app is in front.

### Repository layout

| Path | What is inside |
|---|---|
| `crates/lume-css` | Parser for the CSS subset used by themes (see [`docs/theme-format.md`](docs/theme-format.md)) |
| `crates/lume-strings` | Turns Android `strings.xml` translations into Rust tables at build time |
| `crates/lume-core` | Platform-independent logic: launcher screens, focus, theme, text and GPU renderer, i18n, catalog, persistence |
| `crates/lume-app` | The Android library (`liblume.so`): NativeActivity, EGL, JNI, event loop |
| `tools/lume-desktop` | Desktop viewer and screenshot tool: runs the launcher without a TV |
| `android/` | Minimal Gradle project: manifest, `LumeActivity`, `LumeBridge`, resources, packaging |
| `themes/lume/` | The default theme (MIT) |
| `docker/`, `scripts/` | Reproducible container build, APK checks, font and test-font generators |
| `docs/` | Theme format, translation guide, device notes |
| `openspec/` | Specs, design and task list of the current release |

### Run it on your computer

You do not need a TV to try the interface. This opens a window where the keyboard is the remote (arrow keys, **Enter** = OK, **Esc** = Back, **H** = Home):

```
cargo run -p lume-desktop
cargo run -p lume-desktop -- --locale en --size 1920x1080
```

It renders with the same OpenGL ES 2.0 code as the TV (Mesa's software renderer is enough). To take pictures of a screen instead (`home`, `apps`, `settings`, `about`, `editor`):

```
cargo run -p lume-desktop -- --screenshot home home.png
cargo run -p lume-desktop -- --size 1920x1080 --keys down,right,right --screenshot home focus.png
```

`--keys` presses keys first (`up down left right ok back home`), `--locale` picks the language and `--pseudo-locale` stretches every text by 40% to find layouts that do not fit.

### Build with a container (recommended)

You only need a container runtime. No Rust, NDK or Android SDK on your machine.

The order is also the priority: the script uses the first runtime it finds (unless you set `LUME_CONTAINER`):

1. **wslc:** the tool that ships with WSL on Windows.
2. **Docker:** with BuildKit (`docker buildx`).
3. **Podman:** version 5 or newer.

If none of the three is installed, the script says so and suggests alternatives: update WSL to get `wslc`, install Docker or Podman, point `LUME_CONTAINER` at another Docker-compatible runtime, or build without containers by installing Rust, the Android NDK/SDK and `cargo-ndk` and following `docker/Dockerfile` by hand.

```
scripts/container-build.sh debug      # APK with arm64, armv7 and x86_64
scripts/container-build.sh release    # APK and AAB for arm64 and armv7
```

The result is written to `dist/`. The first build downloads the Android SDK/NDK and Rust and takes a while; later builds reuse the cache. Set `LUME_CONTAINER` to force a runtime.

Install it on your TV or emulator with `adb install -r dist/lume-0.1.0-debug.apk`.

**Signing a release.** Without a key, the release is signed with the debug key and named `...-release-debugsigned.apk`: fine to install yourself, not to publish. To sign with your own key, set these before running the script (they are passed to the build as secrets and never stored in the image):

```
export LUME_KEYSTORE=/path/to/lume.jks
export LUME_KEYSTORE_PASSWORD=...
export LUME_KEY_ALIAS=...
export LUME_KEY_PASSWORD=...
scripts/container-build.sh release    # dist/lume-0.1.0-release.apk
```

Keep the keystore outside the repository and back it up: the same key must sign every update. `LUME_VERSION_NAME=0.2.0` builds another version.

**Checking an APK.** `scripts/verify-apk.py dist/lume-0.1.0-release.apk` checks the size (6 MB), the ABIs, the 16 KB alignment of the native libraries, and that the app asks for neither `INTERNET` nor `QUERY_ALL_PACKAGES`.

### Build without a container

You need:

- Rust (the version and Android targets come from `rust-toolchain.toml`; install [rustup](https://rustup.rs) and it picks them up).
- JDK 17, Android SDK platform 36 and build-tools 36.0.0.
- Android NDK `28.2.13676358` (set `ANDROID_NDK_HOME`).
- `cargo install cargo-ndk --version 4.1.2 --locked`

Then:

```
cd android
./gradlew assembleDebug      # or assembleRelease
```

Gradle calls `cargo ndk` for you and aligns the native libraries to 16 KB pages.

### Run the tests

```
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

The graphics tests need a working EGL (Mesa on Linux); they are skipped when none is available.

### Releases

Pushing a tag such as `v0.1.0` (or `v0.1.0-rc1` for a pre-release) runs `.github/workflows/release.yml`: it builds the signed APK and AAB with the same Dockerfile, runs the checks above and attaches the APK to a GitHub release. It needs these repository secrets: `LUME_KEYSTORE_BASE64` (the keystore, base64-encoded), `LUME_KEYSTORE_PASSWORD`, `LUME_KEY_ALIAS` and `LUME_KEY_PASSWORD`.

### Translations

Translations are standard Android `strings.xml` files in `android/app/src/main/res/values-<locale>/`. See [`docs/translations.md`](docs/translations.md).

### Wallpaper

lume draws one northern-lights photograph behind every screen. How it is processed at build time, how to replace it and where its source and license are recorded: [`docs/wallpaper.md`](docs/wallpaper.md).

### Contributing, license and trademarks

- Code is licensed under **GPL-3.0-or-later** ([`LICENSE`](LICENSE)); themes under **MIT** ([`themes/LICENSE`](themes/LICENSE)).
- The name and logo "lume" are not licensed: see [`TRADEMARKS.md`](TRADEMARKS.md).
- Contributions need a signed CLA: see [`CONTRIBUTING.md`](CONTRIBUTING.md) and [`CLA.md`](CLA.md).
