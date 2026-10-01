// York's own Android build toolchain.
//
// Everything here belongs to York: we fetch the small Google command-line
// compilers once into ~/.york/android (aapt2, d8, zipalign, apksigner) and
// drive them directly — no Gradle, no Android Studio, no full SDK. Then we
// assemble and sign a real .apk ourselves.
//
// Toolchain cache (~/.york/android):
//   build-tools-36/   aapt2.exe, lib/d8.jar, lib/apksigner.jar, zipalign.exe
//   platforms-34/     android.jar (compile + link reference)
//   debug.keystore    locally-generated debug signing key

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context};
use colored::Colorize;

const CACHE_ROOT: &str = ".york/android";
const BUILD_TOOLS_URL: &str = "https://dl.google.com/android/repository/build-tools_r36.1_windows.zip";
const PLATFORM_URL: &str = "https://dl.google.com/android/repository/platform-34-ext7_r03.zip";

fn home() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn cache_dir() -> PathBuf {
    home().join(CACHE_ROOT)
}

fn tool_dir() -> PathBuf {
    cache_dir().join("build-tools-36")
}

fn platform_jar() -> PathBuf {
    cache_dir().join("platforms-34/android.jar")
}

fn keystore() -> PathBuf {
    cache_dir().join("debug.keystore")
}

fn java_bin(tool: &str) -> anyhow::Result<PathBuf> {
    // Prefer the JDK next to a running java, else standard Windows locations.
    if let Ok(java_home) = std::env::var("JAVA_HOME") {
        let p = PathBuf::from(java_home).join("bin").join(tool);
        if p.exists() {
            return Ok(p);
        }
    }
    for root in ["C:/Program Files/Java", "C:/Program Files/Eclipse Adoptium"] {
        if let Ok(entries) = std::fs::read_dir(root) {
            for e in entries.flatten() {
                let p = e.path().join("bin").join(tool);
                if p.exists() {
                    return Ok(p);
                }
            }
        }
    }
    Ok(PathBuf::from(tool)) // rely on PATH
}

fn curl(url: &str, dest: &Path) -> anyhow::Result<()> {
    let status = Command::new("curl.exe")
        .args(["-L", "-C", "-", "-o"])
        .arg(dest)
        .arg(url)
        .args(["--retry", "5", "--retry-delay", "2", "-s"])
        .status()
        .context("spawn curl")?;
    if !status.success() {
        bail!("download failed: {url}");
    }
    Ok(())
}

fn needs_download(zip_path: &Path, _url: &str, min_bytes: u64) -> bool {
    if let Ok(meta) = std::fs::metadata(zip_path) {
        return meta.len() < min_bytes;
    }
    true
}

/// Whether the Android Addon toolchain is already installed in the cache.
pub fn toolchain_installed() -> bool {
    tool_dir().join("aapt2.exe").exists() && platform_jar().exists()
}

/// Install the **York Android Addon** explicitly.
///
/// This is *not* automatic: the addon is downloaded on demand — exactly like
/// installing Android Studio — from the website / GitHub / `york pkg add
/// android-toolchain`. It fetches the small Google command-line compilers
/// once into ~/.york/android (aapt2, d8, zipalign, apksigner) plus
/// android.jar, and creates a local debug keystore.
pub fn install_toolchain() -> anyhow::Result<()> {
    let dir = cache_dir();
    std::fs::create_dir_all(&dir).context("york toolchain dir")?;

    let bt_zip = dir.join("build-tools.zip");
    let pl_zip = dir.join("platform.zip");
    let tools_marker = tool_dir().join("aapt2.exe");

    let need_bt = needs_download(&bt_zip, BUILD_TOOLS_URL, 40_000_000);
    let need_pl = needs_download(&pl_zip, PLATFORM_URL, 40_000_000);

    if !tools_marker.exists() {
        if need_bt {
            println!(
                "{} York Android Addon — Android compiler tools (~60MB from Google) ...",
                "addon:".cyan().bold()
            );
            curl(BUILD_TOOLS_URL, &bt_zip)?;
        }
        let tmp = dir.join("bt_extract");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp)?;
        extract_zip(&bt_zip, &tmp)?;
        // The zip contains a single versioned folder (android-16 etc).
        match std::fs::read_dir(&tmp)?.next() {
            Some(Ok(inner)) => std::fs::rename(inner.path(), &tool_dir())?,
            _ => bail!("build tools archive was empty"),
        }
        std::fs::remove_dir_all(&tmp).ok();
    }

    if !platform_jar().exists() {
        if need_pl {
            println!(
                "{} York Android Addon — platform android.jar (~60MB from Google) ...",
                "addon:".cyan().bold()
            );
            curl(PLATFORM_URL, &pl_zip)?;
        }
        let tmp = dir.join("pl_extract");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp)?;
        extract_zip(&pl_zip, &tmp)?;
        match std::fs::read_dir(&tmp)?.next() {
            Some(Ok(inner)) => {
                let ver_dir = dir.join("platforms-34");
                let _ = std::fs::remove_dir_all(&ver_dir);
                std::fs::rename(inner.path(), &ver_dir)?;
            }
            _ => bail!("platform archive was empty"),
        }
        std::fs::remove_dir_all(&tmp).ok();
    }

    if !keystore().exists() {
        println!("{} local debug signing key ...", "creating:".green().bold());
        let keytool = java_bin("keytool.exe")?;
        let status = Command::new(&keytool)
            .args(["-genkeypair", "-v", "-keystore"])
            .arg(&keystore())
            .args([
                "-storepass", "android",
                "-alias", "androiddebugkey",
                "-keypass", "android",
                "-keyalg", "RSA",
                "-keysize", "2048",
                "-validity", "10000",
                "-dname", "CN=Android Debug,O=York,C=US",
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .context("run keytool")?;
        if !status.success() {
            bail!("keytool failed to create the debug keystore");
        }
    }

    // Tidy: drop the archives once extracted.
    let _ = std::fs::remove_file(&bt_zip);
    let _ = std::fs::remove_file(&pl_zip);

    println!(
        "{} York Android Addon installed — `york mobile app.yk --apk` is now ready.",
        "done:".green().bold()
    );
    Ok(())
}

fn extract_zip(zip_path: &Path, dest: &Path) -> anyhow::Result<()> {
    // tar.exe (bsdtar) handles zip archives on modern Windows.
    let status = Command::new("tar.exe")
        .arg("-xf")
        .arg(zip_path)
        .arg("-C")
        .arg(dest)
        .status()
        .context("spawn tar")?;
    if !status.success() {
        bail!("extract failed: {}", zip_path.display());
    }
    Ok(())
}

/// Build and sign a real APK from a prepared Android project.
///
/// `proj` layout (matches the generated shell):
///   manifest, res/..., assets/ (assets/www/...), java/.../*.java
///
/// Produces `out/app-debug.apk`.
pub fn build_apk(_app_name: &str, proj: &Path, out: &Path) -> anyhow::Result<()> {
    if !toolchain_installed() {
        bail!(
            "{} the York Android Addon is not installed.\n  Install it from: {}  or:\n  {}   ← downloads aapt2, d8, zipalign, apksigner + android.jar (~120MB, one-time)",
            "error:".red().bold(),
            "https://york-lang.org/android".cyan().underline(),
            "york pkg add android-toolchain".green().bold()
        );
    }
    std::fs::create_dir_all(out)?;

    let bt = tool_dir();
    let aapt2 = bt.join("aapt2.exe");
    let zipalign = bt.join("zipalign.exe");
    let android_jar = platform_jar();
    let jar = java_bin("jar.exe")?;
    let d8_jar = bt.join("lib/d8.jar");
    let apksigner = bt.join("apksigner.bat");

    let build = out.join("_york");
    let res_flat = build.join("res-flat");
    let gen_dir = build.join("gen");
    let classes = build.join("classes");
    let dex = build.join("dex");

    for d in [&res_flat, &gen_dir, &classes, &dex] {
        std::fs::create_dir_all(d)?;
    }

    let manifest = proj.join("AndroidManifest.xml");
    if !manifest.exists() {
        bail!("missing AndroidManifest.xml in {}", proj.display());
    }

    // 1. aapt2 compile: flat resource files.
    let status = Command::new(&aapt2)
        .args(["compile", "--dir"])
        .arg(proj.join("res"))
        .args(["-o"])
        .arg(res_flat.join("res.zip"))
        .status()
        .context("aapt2 compile")?;
    if !status.success() {
        bail!("aapt2 compile failed — check res/ files");
    }

    // 2. aapt2 link: merge resources + manifest + assets into base.apk, emit R.java.
    let base_apk = build.join("base.apk");
    let status = Command::new(&aapt2)
        .args(["link", "-o"])
        .arg(&base_apk)
        .args(["-I"])
        .arg(&android_jar)
        .args(["--manifest"])
        .arg(&manifest)
        .args(["-R"])
        .arg(res_flat.join("res.zip"))
        .args(["--java"])
        .arg(&gen_dir)
        .args(["-A"])
        .arg(proj.join("assets"))
        .args(["--auto-add-overlay", "--min-sdk-version", "24", "--target-sdk-version", "34",
               "--version-code", "1", "--version-name", "1.0"])
        .status()
        .context("aapt2 link")?;
    if !status.success() {
        bail!("aapt2 link failed");
    }

    // 3. javac: compile all Java (ours + generated R.java) against android.jar.
    let mut java_sources: Vec<PathBuf> = Vec::new();
    let java_root = proj.join("java");
    if java_root.exists() {
        collect_java(&java_root, &mut java_sources)?;
    }
    let r_java = gen_dir.join(app_pkg_path("york.mobileapp")).join("R.java");
    if r_java.exists() {
        java_sources.push(r_java);
    }
    if java_sources.is_empty() {
        bail!("no java sources in {}", proj.display());
    }
    let javac = java_bin("javac.exe")?;
    let mut cmd = Command::new(&javac);
    cmd.args(["--release", "11", "-nowarn", "-cp"]).arg(&android_jar).args(["-d"]).arg(&classes);
    for src in &java_sources {
        cmd.arg(src);
    }
    let status = cmd.status().context("run javac")?;
    if !status.success() {
        bail!("javac failed — York runtime does not compile");
    }

    // 4. d8: dex the classes.
    let mut d8 = Command::new("java");
    d8.args(["-cp"]).arg(&d8_jar).args(["com.android.tools.r8.D8", "--release", "--min-api", "24", "--lib"])
        .arg(&android_jar)
        .args(["--output"])
        .arg(&dex);
    append_classes(&classes, &mut d8)?;
    let status = d8.status().context("run d8")?;
    if !status.success() {
        bail!("d8 dexing failed");
    }

    // 5. Merge classes.dex into base.apk (jar keeps the zip valid for zipalign).
    let merged = build.join("merged.apk");
    std::fs::copy(&base_apk, &merged)?;
    let status = Command::new(&jar)
        .args(["uf"])
        .arg(&merged)
        .args(["-C"])
        .arg(&dex)
        .arg("classes.dex")
        .status()
        .context("merge classes.dex")?;
    if !status.success() {
        bail!("jar merge failed");
    }

    // 6. zipalign (4-byte) for performant installs.
    let aligned = build.join("aligned.apk");
    let status = Command::new(&zipalign)
        .args(["-f", "4"])
        .arg(&merged)
        .arg(&aligned)
        .status()
        .context("zipalign")?;
    if !status.success() {
        bail!("zipalign failed");
    }

    // 7. Sign with our local debug key (v2/v3).
    let final_apk = out.join("app-debug.apk");
    let status = Command::new(&apksigner)
        .args(["sign", "--ks"])
        .arg(&keystore())
        .args(["--ks-pass", "pass:android", "--key-pass", "pass:android", "--out"])
        .arg(&final_apk)
        .arg(&aligned)
        .status()
        .context("apksigner")?;
    if !status.success() {
        bail!("apk signing failed");
    }

    let _ = std::fs::remove_dir_all(&build);
    Ok(())
}

fn app_pkg_path(pkg: &str) -> PathBuf {
    pkg.split('.').collect()
}

fn collect_java(dir: &Path, out: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_java(&path, out)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("java") {
            out.push(path);
        }
    }
    Ok(())
}

fn append_classes(classes: &Path, cmd: &mut Command) -> anyhow::Result<()> {
    fn walk(dir: &Path, cmd: &mut Command, out: &mut Vec<PathBuf>) -> anyhow::Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                walk(&path, cmd, out)?;
            } else if path.extension().and_then(|e| e.to_str()) == Some("class") {
                out.push(path);
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    walk(classes, cmd, &mut files)?;
    for f in files {
        cmd.arg(f);
    }
    Ok(())
}