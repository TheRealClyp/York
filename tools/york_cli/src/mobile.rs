// York mobile app bundler.
//
// `york mobile <app.yk>` runs a York program in a staging directory. The
// program emits a mobile web app using the built-in mobile UI helpers
// (mobile_head, mobile_row, mobile_card, mobile_text, ...) into index.html.
// This command then packages the result into a cross-platform mobile app:
//   - an installable PWA (manifest.webmanifest + service worker + icons)
//   - an Android shell with York's own runtime bridge — compiled to a real
//     signed .apk by York's own toolchain (see android_build.rs) when --apk
//   - an iOS Xcode project when --platform ios
// so the same York source can go to the Play Store, the App Store, or the
// home screen. `window.York` (Core 12 device features) is injected into
// every app.

use std::path::Path;

use anyhow::{bail, Context};
use colored::Colorize;

use crate::{android_build, pipeline};

/// Which native shell(s) to generate.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Android,
    Ios,
    Both,
}

impl Platform {
    fn wants_android(self) -> bool {
        matches!(self, Platform::Android | Platform::Both)
    }
    fn wants_ios(self) -> bool {
        matches!(self, Platform::Ios | Platform::Both)
    }
}

/// Build a mobile app from a York source file.
pub fn cmd_mobile(
    file: &Path,
    name: Option<&str>,
    out: &Path,
    icon: Option<&Path>,
    serve: bool,
    platform: Platform,
    apk: bool,
) -> anyhow::Result<()> {
    if out.exists() {
        std::fs::remove_dir_all(out).context("cleaning output dir")?;
    }
    std::fs::create_dir_all(out).context("creating output dir")?;

    // York programs use relative paths via write_file / append_file, so run
    // the compiled binary with the output dir as its working directory.
    let c_source = pipeline::compile_to_c(file)?;
    let stage = std::env::temp_dir().join(format!("york_mobile_{}", std::process::id()));
    std::fs::create_dir_all(&stage).context("staging dir")?;
    let exe = stage.join("york_mobile_out");

    york_codegen_c::compile_c(&c_source, exe.to_str().unwrap())
        .map_err(|e| anyhow::anyhow!("{e}"))
        .context("C compilation failed")?;

    let exe_path = pipeline::resolve_exe(&exe);
    let status = std::process::Command::new(&exe_path)
        .current_dir(out)
        .status()
        .with_context(|| format!("failed to execute `{}`", exe_path.display()))?;
    if !status.success() {
        bail!("app program exited with status {status}");
    }

    let app_name = name.unwrap_or_else(|| "York App");
    write_web_assets(out)?;
    let _ = std::fs::remove_dir_all(&stage);

    println!("{} app web output  -> {}", "packaged:".green().bold(), out.display());
    println!("  index.html, app.js, style.css + runtime (York.*) are in the output directory.");

    write_pwa(app_name, out)?;

    if platform.wants_android() {
        write_android_shell(app_name, out, icon)?;
        println!();
        println!("{}", "mobile app finished (Android)".green().bold());
        println!("    {}  <- open in Android Studio -> Run", out.join("android").display());
        println!("    {}  Java runtime bridge (York.*)", out.join("android/app/src/main/java/york/mobileapp/YorkBridge.java").display());
        if apk {
            let apk_out = out.join("apk");
            println!("{} compiling real APK with York's own toolchain ...", "building:".green().bold());
            android_build::build_apk(app_name, &out.join("android/app/src/main"), &apk_out)?;
            println!("{} {}", "apk:", apk_out.join("app-debug.apk").display().to_string().green().bold());
            println!("    install on a phone with:  adb install {}", apk_out.join("app-debug.apk").display());
        }
    }

    if platform.wants_ios() {
        write_ios_shell(app_name, out)?;
        println!();
        println!("{}", "mobile app finished (iOS)".green().bold());
        println!("    {}  <- open in Xcode on a Mac -> Run", out.join("ios").display());
        println!("    {}  Swift runtime bridge (York.*)", out.join("ios/YorkApp/YorkBridge.swift").display());
    }

    println!();
    println!("  PWA (install to home screen / demo on any phone)");
    println!("    {}  manifest + icons", out.join("manifest.webmanifest").display());
    println!("    {}  offline service worker", out.join("sw.js").display());
    if serve {
        serve_web(out)?;
    }
    Ok(())
}

/// Minimal local HTTP server: preview the web app on a phone over LAN.
fn serve_web(out: &Path) -> anyhow::Result<()> {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    let listener = TcpListener::bind("0.0.0.0:8787").context("bind 0.0.0.0:8787")?;
    listener
        .local_addr()
        .context("local addr")?;

    local_ips();
    println!("{}", "serving web app on http://0.0.0.0:8787".green().bold());
    println!("  open this from your phone (same Wi-Fi) using the LAN IP printed above.");
    println!("  Ctrl+C to stop.");

    for stream in listener.incoming() {
        let Ok(mut s) = stream else { continue };
        let mut buf = [0u8; 8192];
        let _ = s.read(&mut buf);
        let req = String::from_utf8_lossy(&buf);
        let mut path = req
            .lines()
            .next()
            .and_then(|l| l.split_whitespace().nth(1))
            .unwrap_or("/")
            .to_string();
        if path == "/" {
            path = "/index.html".to_string();
        }
        let path = path.trim_start_matches('/');
        let file = out.join(path);
        let (status, body, ctype) = if file.is_file() {
            let b = std::fs::read(&file).unwrap_or_default();
            let ctype = match file.extension().and_then(|e| e.to_str()) {
                Some("html") => "text/html",
                Some("css") => "text/css",
                Some("js") => "application/javascript",
                Some("webmanifest") | Some("json") => "application/json",
                Some("svg") => "image/svg+xml",
                Some("png") => "image/png",
                _ => "application/octet-stream",
            };
            ("200 OK", b, ctype)
        } else {
            (
                "404 Not Found",
                b"not found".to_vec(),
                "text/plain",
            )
        };
        let header = format!(
            "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n",
            body.len()
        );
        let _ = s.write_all(header.as_bytes());
        let _ = s.write_all(&body);
    }
    Ok(())
}

/// Print LAN addresses the phone can use to reach this machine.
fn local_ips() {
    let _ = std::process::Command::new("ipconfig")
        .output()
        .map(|o| {
            let text = String::from_utf8_lossy(&o.stdout);
            for line in text.lines() {
                let l = line.trim();
                if l.contains("IPv4") || l.contains("IPv6") {
                        if let Some(addr) = l.split(':').last() {
                            println!("  LAN: http://{}:8787", addr.trim());
                        }
                    }
            }
        });
}

/// Scaffold a new mobile app project: source + instructions.
pub fn cmd_new_mobile(name: &str) -> anyhow::Result<()> {
    let root = std::env::current_dir()?.join(name);
    if root.exists() {
        bail!("directory `{}` already exists", name);
    }
    std::fs::create_dir_all(&root)?;
    std::fs::create_dir_all(root.join("src"))?;

    std::fs::write(
        root.join("src").join("app.yk"),
        include_str!("template_mobile_app.yk"),
    )?;
    std::fs::write(
        root.join("README.md"),
        format!(
            "# {name}\n\nA York mobile app.\n\n```\nyork mobile src/app.yk --out build\n```\nThen open `build/android` in Android Studio, or host `build/` to install the PWA.\n"
        ),
    )?;

    println!("{} created mobile app `{}`", "created:".green().bold(), name);
    println!("  cd {name}");
    println!("  {}  ← build the mobile app", "york mobile src/app.yk --out build".cyan());
    println!("  {}  ← open in Android Studio and run", "build/android".cyan());
    Ok(())
}

/// Ensure a minimal index.html exists if the program didn't create one.
fn write_web_assets(out: &Path) -> anyhow::Result<()> {
    // York's device runtime is part of every app.
    std::fs::write(out.join("york-mobile.js"), include_str!("mobile_runtime.js"))
        .context("write york-mobile.js")?;

    if !out.join("index.html").exists() {
        std::fs::write(
            out.join("index.html"),
            "<!DOCTYPE html><html><head><meta charset='utf-8'><meta name='viewport' content='width=device-width, initial-scale=1'><title>York App</title><link rel='stylesheet' href='style.css'></head><body><div id='app'><p>Your app produced no index.html. Use the mobile UI helpers in your York program.</p></div><script src='york-mobile.js'></script><script src='app.js'></script></body></html>",
        )
            .context("write default index.html")?;
    }
    if !out.join("style.css").exists() {
        std::fs::write(out.join("style.css"), "body { font-family: system-ui; margin: 0; padding: 16px; }\n")
            .context("write default style.css")?;
    }
    if !out.join("app.js").exists() {
        std::fs::write(out.join("app.js"), "// York mobile app\n")
            .context("write default app.js")?;
    }
    Ok(())
}

/// Inject `<script src='york-mobile.js'>` before app.js for programs that
/// create their own index.html.
fn inject_runtime(html: &str) -> String {
    if html.contains("york-mobile.js") {
        return html.to_string();
    }
    let tag = "<script src='york-mobile.js'></script>";
    if let Some(pos) = html.rfind("<script") {
        let mut out = String::with_capacity(html.len() + tag.len() + 1);
        out.push_str(&html[..pos]);
        out.push_str(tag);
        out.push('\n');
        out.push_str(&html[pos..]);
        out
    } else {
        format!("{html}<script src='york-mobile.js'></script>")
    }
}

/// PWA bits: manifest, service worker, icons.
fn write_pwa(app_name: &str, out: &Path) -> anyhow::Result<()> {
    std::fs::write(
        out.join("manifest.webmanifest"),
        format!(
            r##"{{
  "name": "{app_name}",
  "short_name": "{app_name}",
  "start_url": ".",
  "display": "standalone",
  "background_color": "#04070d",
  "theme_color": "#22d3ee",
  "orientation": "any",
  "icons": [
    {{ "src": "icon-192.svg", "sizes": "192x192", "type": "image/svg+xml" }},
    {{ "src": "icon-512.svg", "sizes": "512x512", "type": "image/svg+xml" }}
  ]
}}
"##
        ),
    )
    .context("write manifest")?;

    std::fs::write(
        out.join("sw.js"),
        "const CACHE = 'york-v1';\n\
self.addEventListener('install', (e) => {\n\
  e.waitUntil(caches.open(CACHE).then((c) => c.addAll(['./', './index.html', './style.css', './york-mobile.js', './app.js'])));\n\
  self.skipWaiting();\n\
});\n\
self.addEventListener('activate', (e) => {\n\
  e.waitUntil(\n\
    caches.keys().then((keys) => Promise.all(keys.filter((k) => k !== CACHE).map((k) => caches.delete(k))))\n\
  );\n\
  self.clients.claim();\n\
});\n\
self.addEventListener('fetch', (e) => {\n\
  if (e.request.method !== 'GET') return;\n\
  e.respondWith(\n\
    caches.match(e.request).then((hit) => hit || fetch(e.request).then((res) => {\n\
      if (!res || res.status !== 200) return res;\n\
      const clone = res.clone();\n\
      caches.open(CACHE).then((c) => c.put(e.request, clone));\n\
      return res;\n\
    }))\n\
  );\n\
});\n",
    )
    .context("write service worker")?;

    write_icon(out, "icon-192.svg", 192)?;
    write_icon(out, "icon-512.svg", 512)?;

    // index.html: reference manifest + register the worker + runtime.
    let path = out.join("index.html");
    let html = std::fs::read_to_string(&path)?;
    let html = inject_runtime(&html);
    let head = format!(
        "<link rel='manifest' href='./manifest.webmanifest'>\n<meta name='theme-color' content='#22d3ee'>\n<meta name='mobile-web-app-capable' content='yes'>\n<meta name='apple-mobile-web-app-capable' content='yes'>\n<meta name='apple-mobile-web-app-status-bar-style' content='black-translucent'>\n<link rel='apple-touch-icon' href='./icon-192.svg'>\n"
    );
    let with_tags = if html.contains("<head>") {
        html.replacen("<head>", &format!("<head>\n{head}"), 1)
    } else {
        format!("<!DOCTYPE html><html><head><meta charset='utf-8'><meta name='viewport' content='width=device-width, initial-scale=1'>{head}</head><body>{html}</body></html>")
    };
    let with_sw = if with_tags.contains("serviceWorker.register") {
        with_tags
    } else if with_tags.contains("</body>") {
        with_tags.replacen(
            "</body>",
            "<script>if ('serviceWorker' in navigator) navigator.serviceWorker.register('./sw.js');</script></body>",
            1,
        )
    } else {
        format!("{with_tags}<script>if ('serviceWorker' in navigator) navigator.serviceWorker.register('./sw.js');</script>")
    };
    std::fs::write(path, with_sw).context("update index.html for PWA")?;
    Ok(())
}

fn write_icon(out: &Path, file: &str, _size: u32) -> anyhow::Result<()> {
    std::fs::write(
        out.join(file),
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="512" height="512" viewBox="0 0 512 512">
  <rect width="512" height="512" rx="96" fill="#04070d"/>
  <rect x="24" y="24" width="464" height="464" rx="80" fill="none" stroke="#22d3ee" stroke-width="8"/>
  <text x="256" y="330" font-size="300" text-anchor="middle" fill="#22d3ee" font-family="monospace" font-weight="900">Y</text>
</svg>
"##,
    )
    .with_context(|| format!("write {file}"))?;
    Ok(())
}

/// Minimal Android Gradle shell that hosts the web app in a WebView.
fn write_android_shell(app_name: &str, out: &Path, icon: Option<&Path>) -> anyhow::Result<()> {
    let pkg_path = "york/mobileapp";
    let root = out.join("android");
    std::fs::create_dir_all(root.join("app/src/main")).context("android dirs")?;
    std::fs::create_dir_all(root.join("gradle/wrapper")).context("android wrapper dir")?;

    std::fs::write(
        root.join("settings.gradle"),
        "pluginManagement { repositories { google(); mavenCentral(); gradlePluginPortal() } }\ndependencyResolutionManagement { repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS); repositories { google(); mavenCentral() } }\nrootProject.name = \"YorkApp\"\ninclude ':app'\n",
    )?;
    std::fs::write(
        root.join("build.gradle"),
        "plugins { id 'com.android.application' version '8.2.2' apply false }\n",
    )?;
    std::fs::write(
        root.join("gradle.properties"),
        "org.gradle.jvmargs=-Xmx2048m\nandroid.useAndroidX=true\n",
    )?;
    std::fs::write(
        root.join("app/build.gradle"),
        r#"plugins { id 'com.android.application' }
android {
    namespace 'york.mobileapp'
    compileSdk 34
    defaultConfig {
        applicationId "york.mobileapp"
        minSdk 24
        targetSdk 34
        versionCode 1
        versionName "1.0"
    }
    buildTypes { release { minifyEnabled false } }
}
dependencies { }
"#,
    )?;

    // Activity + York runtime bridge (+ our tiny interface)
    let src_dir = root.join("app/src/main/java").join(pkg_path);
    std::fs::create_dir_all(&src_dir)?;
    std::fs::write(
        src_dir.join("MainActivity.java"),
        include_str!("template_android/MainActivity.java"),
    )?;
    std::fs::write(
        src_dir.join("YorkBridge.java"),
        include_str!("template_android/YorkBridge.java"),
    )?;
    std::fs::write(
        src_dir.join("YorkCallable.java"),
        include_str!("template_android/YorkCallable.java"),
    )?;

    std::fs::write(
        root.join("app/src/main/AndroidManifest.xml"),
        r#"<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="york.mobileapp">
  <uses-permission android:name="android.permission.INTERNET"/>
  <uses-permission android:name="android.permission.ACCESS_NETWORK_STATE"/>
  <uses-permission android:name="android.permission.VIBRATE"/>
  <uses-permission android:name="android.permission.ACCESS_FINE_LOCATION"/>
  <uses-permission android:name="android.permission.ACCESS_COARSE_LOCATION"/>
  <application android:label="@string/app_name" android:icon="@mipmap/ic_launcher"
      android:theme="@style/AppTheme" android:hardwareAccelerated="true"
      android:usesCleartextTraffic="true">
    <activity android:name=".MainActivity" android:exported="true"
        android:configChanges="orientation|screenSize|screenLayout|keyboardHidden|locale">
      <intent-filter>
        <action android:name="android.intent.action.MAIN"/>
        <category android:name="android.intent.category.LAUNCHER"/>
      </intent-filter>
    </activity>
  </application>
</manifest>
"#,
    )?;

    // res: theme + splash + adaptive launcher icons (VectorDrawable XML, not SVG).
    let res = root.join("app/src/main/res");
    std::fs::create_dir_all(res.join("values")).context("res values")?;
    std::fs::create_dir_all(res.join("drawable")).context("res drawable")?;
    std::fs::create_dir_all(res.join("mipmap-anydpi-v26")).context("res mipmap")?;
    std::fs::write(
        res.join("values/strings.xml"),
        format!("<resources><string name=\"app_name\">{app_name}</string></resources>"),
    )?;
    std::fs::write(
        res.join("values/colors.xml"),
        "<resources><color name=\"bg\">#04070d</color><color name=\"accent\">#22d3ee</color><color name=\"splash_bg\">#04070d</color></resources>",
    )?;
    std::fs::write(
        res.join("values/styles.xml"),
        r#"<resources><style name="AppTheme" parent="android:Theme.Material.NoActionBar">
<item name="android:windowBackground">@color/splash_bg</item>
<item name="android:statusBarColor">@color/splash_bg</item>
<item name="android:navigationBarColor">@color/splash_bg</item>
</style></resources>"#,
    )?;
    // Custom icon (if a path was given) is still copied for the web UI; the
    // Android launcher always uses the adaptive vector below so it builds.
    if let Some(icon_path) = icon {
        std::fs::create_dir_all(res.join("mipmap-hdpi"))?;
        std::fs::write(res.join("mipmap-hdpi/ic_launcher.svg"), std::fs::read(icon_path)?)?;
    }
    std::fs::write(
        res.join("drawable/ic_launcher_foreground.xml"),
        r##"<vector xmlns:android="http://schemas.android.com/apk/res/android"
    android:width="108dp" android:height="108dp"
    android:viewportWidth="108" android:viewportHeight="108">
    <path android:fillColor="#22d3ee"
        android:pathData="M54,30 L70,62 L70,78 L54,78 L54,66 L54,66 L38,78 L38,62 Z"
        android:strokeColor="#00000000"/>
</vector>
"##,
    )?;
    std::fs::write(
        res.join("mipmap-anydpi-v26/ic_launcher.xml"),
        r#"<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">
    <background android:drawable="@color/bg" />
    <foreground android:drawable="@drawable/ic_launcher_foreground" />
</adaptive-icon>
"#,
    )?;
    // Legacy fallback icon (reference the same adaptive resource).
    std::fs::create_dir_all(res.join("drawable-v24"))?;
    std::fs::write(
        res.join("drawable-v24/ic_launcher_legacy.xml"),
        r##"<vector xmlns:android="http://schemas.android.com/apk/res/android"
    android:width="48dp" android:height="48dp"
    android:viewportWidth="48" android:viewportHeight="48">
    <path android:fillColor="#04070d" android:pathData="M0,0h48v48h-48z"/>
    <path android:fillColor="#22d3ee"
        android:pathData="M24,13 L31,29 L31,35 L24,35 L24,29 L17,35 L17,29 Z"/>
</vector>
"##,
    )?;

    // Copy web app into android assets.
    let assets = root.join("app/src/main/assets/www");
    std::fs::create_dir_all(&assets)?;
    for f in ["index.html", "style.css", "york-mobile.js", "app.js", "manifest.webmanifest", "sw.js", "icon-192.svg", "icon-512.svg"] {
        let src = out.join(f);
        if src.exists() {
            std::fs::copy(&src, assets.join(f)).context("copy asset")?;
        }
    }

    Ok(())
}

/// Complete iOS Xcode project: WKWebView shell + York.* Swift bridge.
#[allow(clippy::too_many_lines)]
fn write_ios_shell(app_name: &str, out: &Path) -> anyhow::Result<()> {
    let root = out.join("ios");
    let app = root.join("YorkApp");
    std::fs::create_dir_all(app.join("YorkApp/Assets.xcassets/AppIcon.appiconset")).context("ios dirs")?;
    std::fs::create_dir_all(app.join("YorkApp/www")).context("ios www")?;

    // Swift app entry
    std::fs::write(
        app.join("YorkApp/YorkApp.swift"),
        r#"import SwiftUI

@main
struct YorkApp: App {
    var body: some Scene {
        WindowGroup {
            ContentView()
                .preferredColorScheme(.dark)
        }
    }
}
"#,
    )?;

    // SwiftUI WKWebView wrapper
    std::fs::write(
        app.join("YorkApp/ContentView.swift"),
        r#"import SwiftUI
import WebKit

struct ContentView: UIViewRepresentable {
    func makeUIView(context: Context) -> WKWebView {
        let config = WKWebViewConfiguration()
        config.websiteDataStore = .default()
        let web = WKWebView(frame: .zero, configuration: config)
        web.isOpaque = false
        web.backgroundColor = UIColor(red: 0.016, green: 0.027, blue: 0.051, alpha: 1)
        web.scrollView.bounces = false
        web.navigationDelegate = context.coordinator

        // York runtime bridge: window.webkit.messageHandlers.york
        let bridge = YorkBridge(web: web)
        config.userContentController.add(bridge, name: "york")

        if let path = Bundle.main.path(forResource: "index", ofType: "html", inDirectory: "www") {
            let url = URL(fileURLWithPath: path)
            web.loadFileURL(url, allowingReadAccessTo: url.deletingLastPathComponent())
        }
        return web
    }

    func updateUIView(_ uiView: WKWebView, context: Context) {}

    func makeCoordinator() -> Coordinator { Coordinator(self) }

    class Coordinator: NSObject, WKNavigationDelegate {
        var parent: ContentView
        init(_ p: ContentView) { parent = p }
        func webView(_ web: WKWebView, decidePolicyFor nav: WKNavigationAction, decisionHandler: @escaping (WKNavigationActionPolicy) -> Void) {
            if let url = nav.request.url, nav.navigationType == .linkActivated, url.scheme == "http" || url.scheme == "https" {
                UIApplication.shared.open(url)
                decisionHandler(.cancel)
                return
            }
            decisionHandler(.allow)
        }
    }
}
"#,
    )?;

    // Swift runtime bridge implementing the Core 12 via WKScriptMessageHandler
    std::fs::write(
        app.join("YorkApp/YorkBridge.swift"),
        include_str!("template_ios/YorkBridge.swift"),
    )?;

    // Info.plist
    std::fs::write(
        app.join("YorkApp/Info.plist"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key><string>en</string>
  <key>CFBundleDisplayName</key><string>APP_NAME</string>
  <key>CFBundleExecutable</key><string>$(EXECUTABLE_NAME)</string>
  <key>CFBundleIdentifier</key><string>$(PRODUCT_BUNDLE_IDENTIFIER)</string>
  <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
  <key>CFBundleName</key><string>$(PRODUCT_NAME)</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>1.0</string>
  <key>CFBundleVersion</key><string>1</string>
  <key>LSRequiresIPhoneOS</key><true/>
  <key>UILaunchScreen</key><dict>
    <key>UIColorName</key><string>LaunchBackground</string>
  </dict>
  <key>UISupportedInterfaceOrientations</key>
  <array>
    <string>UIInterfaceOrientationPortrait</string>
    <string>UIInterfaceOrientationLandscapeLeft</string>
    <string>UIInterfaceOrientationLandscapeRight</string>
  </array>
  <key>NSLocationWhenInUseUsageDescription</key>
  <string>York uses your location for the app you asked for.</string>
  <key>NSMotionUsageDescription</key>
  <string>York uses the accelerometer for the app you asked for.</string>
  <key>NSCameraUsageDescription</key>
  <string>York uses the camera so your app can take photos.</string>
</dict>
</plist>
"#.replace("APP_NAME", app_name),
    )?;

    // Asset catalog
    std::fs::write(
        app.join("YorkApp/Assets.xcassets/Contents.json"),
        r#"{ "info": { "author": "york", "version": 1 } }
"#,
    )?;
    std::fs::write(
        app.join("YorkApp/Assets.xcassets/AppIcon.appiconset/Contents.json"),
        r#"{
  "images" : [
    { "idiom" : "universal", "platform" : "ios", "size" : "1024x1024" }
  ],
  "info" : { "author" : "york", "version" : 1 }
}
"#,
    )?;
    let launch_bg = app.join("YorkApp/Assets.xcassets/LaunchBackground.colorset");
    std::fs::create_dir_all(&launch_bg)?;
    std::fs::write(
        launch_bg.join("Contents.json"),
        r#"{
  "colors" : [ { "color" : { "color-space" : "srgb", "components" : { "alpha" : "1.000", "blue" : "0.051", "green" : "0.027", "red" : "0.016" } }, "idiom" : "universal" } ],
  "info" : { "author" : "york", "version" : 1 }
}
"#,
    )?;

    // xcodeproj
    write_pbxproj(app_name, &app, &root)?;

    // www copy (shared across all platforms).
    let www = app.join("YorkApp/www");
    for f in ["index.html", "style.css", "york-mobile.js", "app.js", "manifest.webmanifest", "sw.js", "icon-192.svg", "icon-512.svg"] {
        let src = out.join(f);
        if src.exists() {
            std::fs::copy(&src, www.join(f)).context("copy ios asset")?;
        }
    }

    Ok(())
}

/// Write the project.pbxproj (single-target SwiftUI app) from a validated template.
fn write_pbxproj(app_name: &str, _app: &Path, root: &Path) -> anyhow::Result<()> {
    let bundle_safe = sanitize_bundle(app_name);
    let pbx = format!(
        r#"// !$*UTF8*$!
{{
	archiveVersion = 1;
	classes = {{
	}};
	objectVersion = 56;
	objects = {{

/* Begin PBXBuildFile section */
		B10000000000000000000001 /* YorkApp.swift in Sources */ = {{isa = PBXBuildFile; fileRef = F10000000000000000000001 /* YorkApp.swift */; }};
		B10000000000000000000002 /* ContentView.swift in Sources */ = {{isa = PBXBuildFile; fileRef = F10000000000000000000002 /* ContentView.swift */; }};
		B10000000000000000000003 /* YorkBridge.swift in Sources */ = {{isa = PBXBuildFile; fileRef = F10000000000000000000003 /* YorkBridge.swift */; }};
		B10000000000000000000004 /* Assets.xcassets in Resources */ = {{isa = PBXBuildFile; fileRef = F10000000000000000000004 /* Assets.xcassets */; }};
		B10000000000000000000005 /* www in Resources */ = {{isa = PBXBuildFile; fileRef = F10000000000000000000006 /* www */; }};
/* End PBXBuildFile section */

/* Begin PBXFileReference section */
		F10000000000000000000001 /* YorkApp.swift */ = {{isa = PBXFileReference; lastKnownFileType = sourcecode.swift; path = YorkApp.swift; sourceTree = "<group>"; }};
		F10000000000000000000002 /* ContentView.swift */ = {{isa = PBXFileReference; lastKnownFileType = sourcecode.swift; path = ContentView.swift; sourceTree = "<group>"; }};
		F10000000000000000000003 /* YorkBridge.swift */ = {{isa = PBXFileReference; lastKnownFileType = sourcecode.swift; path = YorkBridge.swift; sourceTree = "<group>"; }};
		F10000000000000000000004 /* Assets.xcassets */ = {{isa = PBXFileReference; lastKnownFileType = folder.assetcatalog; path = Assets.xcassets; sourceTree = "<group>"; }};
		F10000000000000000000005 /* Info.plist */ = {{isa = PBXFileReference; lastKnownFileType = text.plist.xml; path = Info.plist; sourceTree = "<group>"; }};
		F10000000000000000000006 /* www */ = {{isa = PBXFileReference; lastKnownFileType = folder; path = www; sourceTree = "<group>"; }};
		F10000000000000000000010 /* YorkApp.app */ = {{isa = PBXFileReference; explicitFileType = wrapper.application; includeInIndex = 0; path = YorkApp.app; sourceTree = BUILT_PRODUCTS_DIR; }};
/* End PBXFileReference section */

/* Begin PBXFrameworksBuildPhase section */
		C20000000000000000000001 /* Frameworks */ = {{
			isa = PBXFrameworksBuildPhase;
			buildActionMask = 2147483647;
			files = (
			);
			runOnlyForDeploymentPostprocessing = 0;
		}};
/* End PBXFrameworksBuildPhase section */

/* Begin PBXGroup section */
		G10000000000000000000001 = {{
			isa = PBXGroup;
			children = (
				G10000000000000000000002 /* YorkApp */,
				G10000000000000000000003 /* Products */,
			);
			sourceTree = "<group>";
		}};
		G10000000000000000000002 /* YorkApp */ = {{
			isa = PBXGroup;
			children = (
				G10000000000000000000004 /* YorkApp */,
			);
			path = YorkApp;
			sourceTree = "<group>";
		}};
		G10000000000000000000004 /* YorkApp */ = {{
			isa = PBXGroup;
			children = (
				F10000000000000000000001 /* YorkApp.swift */,
				F10000000000000000000002 /* ContentView.swift */,
				F10000000000000000000003 /* YorkBridge.swift */,
				F10000000000000000000004 /* Assets.xcassets */,
				F10000000000000000000005 /* Info.plist */,
				F10000000000000000000006 /* www */,
			);
			path = YorkApp;
			sourceTree = "<group>";
		}};
		G10000000000000000000003 /* Products */ = {{
			isa = PBXGroup;
			children = (
				F10000000000000000000010 /* YorkApp.app */,
			);
			name = Products;
			sourceTree = "<group>";
		}};
/* End PBXGroup section */

/* Begin PBXNativeTarget section */
		T10000000000000000000001 /* YorkApp */ = {{
			isa = PBXNativeTarget;
			buildConfigurationList = C30000000000000000000001 /* Build configuration list for PBXNativeTarget "YorkApp" */;
			buildPhases = (
				C10000000000000000000001 /* Sources */,
				C20000000000000000000001 /* Frameworks */,
				C20000000000000000000002 /* Resources */,
			);
			buildRules = (
			);
			dependencies = (
			);
			name = YorkApp;
			productName = YorkApp;
			productReference = F10000000000000000000010 /* YorkApp.app */;
			productType = "com.apple.product-type.application";
		}};
/* End PBXNativeTarget section */

/* Begin PBXProject section */
		P10000000000000000000001 /* Project object */ = {{
			isa = PBXProject;
			attributes = {{
				BuildIndependentTargetsInParallel = 1;
				LastSwiftUpdateCheck = 1500;
				LastUpgradeCheck = 1500;
				TargetAttributes = {{
					T10000000000000000000001 = {{
						CreatedOnToolsVersion = 15.0;
					}};
				}};
			}};
			buildConfigurationList = C30000000000000000000002 /* Build configuration list for PBXProject "YorkApp" */;
			compatibilityVersion = "Xcode 14.0";
			developmentRegion = en;
			hasScannedForEncodings = 0;
			knownRegions = (
				en,
				Base,
			);
			mainGroup = G10000000000000000000001;
			productRefGroup = G10000000000000000000003 /* Products */;
			projectDirPath = "";
			projectRoot = "";
			targets = (
				T10000000000000000000001 /* YorkApp */,
			);
		}};
/* End PBXProject section */

/* Begin PBXResourcesBuildPhase section */
		C20000000000000000000002 /* Resources */ = {{
			isa = PBXResourcesBuildPhase;
			buildActionMask = 2147483647;
			files = (
				B10000000000000000000004 /* Assets.xcassets in Resources */,
				B10000000000000000000005 /* www in Resources */,
			);
			runOnlyForDeploymentPostprocessing = 0;
		}};
/* End PBXResourcesBuildPhase section */

/* Begin PBXSourcesBuildPhase section */
		C10000000000000000000001 /* Sources */ = {{
			isa = PBXSourcesBuildPhase;
			buildActionMask = 2147483647;
			files = (
				B10000000000000000000001 /* YorkApp.swift in Sources */,
				B10000000000000000000002 /* ContentView.swift in Sources */,
				B10000000000000000000003 /* YorkBridge.swift in Sources */,
			);
			runOnlyForDeploymentPostprocessing = 0;
		}};
/* End PBXSourcesBuildPhase section */

/* Begin XCBuildConfiguration section */
		D10000000000000000000001 /* Debug */ = {{
			isa = XCBuildConfiguration;
			buildSettings = {{
				CODE_SIGN_STYLE = Automatic;
				CURRENT_PROJECT_VERSION = 1;
				ENABLE_TESTABILITY = YES;
				GENERATE_INFOPLIST_FILE = NO;
				INFOPLIST_FILE = YorkApp/YorkApp/Info.plist;
				IPHONEOS_DEPLOYMENT_TARGET = 15.0;
				LD_RUNPATH_SEARCH_PATHS = (
					"$(inherited)",
					"@executable_path/Frameworks",
				);
				MARKETING_VERSION = 1.0;
				ONLY_ACTIVE_ARCH = YES;
				PRODUCT_BUNDLE_IDENTIFIER = "{bundle_safe}";
				PRODUCT_NAME = "$(TARGET_NAME)";
				SWIFT_EMIT_LOC_STRINGS = YES;
				SWIFT_OPTIMIZATION_LEVEL = "-Onone";
				SWIFT_VERSION = 5.0;
				TARGETED_DEVICE_FAMILY = "1,2";
			}};
			name = Debug;
		}};
		D10000000000000000000002 /* Release */ = {{
			isa = XCBuildConfiguration;
			buildSettings = {{
				CODE_SIGN_STYLE = Automatic;
				CURRENT_PROJECT_VERSION = 1;
				GENERATE_INFOPLIST_FILE = NO;
				INFOPLIST_FILE = YorkApp/YorkApp/Info.plist;
				IPHONEOS_DEPLOYMENT_TARGET = 15.0;
				LD_RUNPATH_SEARCH_PATHS = (
					"$(inherited)",
					"@executable_path/Frameworks",
				);
				MARKETING_VERSION = 1.0;
				PRODUCT_BUNDLE_IDENTIFIER = "{bundle_safe}";
				PRODUCT_NAME = "$(TARGET_NAME)";
				SWIFT_COMPILATION_MODE = wholemodule;
				SWIFT_EMIT_LOC_STRINGS = YES;
				SWIFT_VERSION = 5.0;
				TARGETED_DEVICE_FAMILY = "1,2";
			}};
			name = Release;
		}};
/* End XCBuildConfiguration section */

/* Begin XCConfigurationList section */
		C30000000000000000000001 /* Build configuration list for PBXNativeTarget "YorkApp" */ = {{
			isa = XCConfigurationList;
			buildConfigurations = (
				D10000000000000000000001 /* Debug */,
				D10000000000000000000002 /* Release */,
			);
			defaultConfigurationIsVisible = 0;
			defaultConfigurationName = Release;
		}};
		C30000000000000000000002 /* Build configuration list for PBXProject "YorkApp" */ = {{
			isa = XCConfigurationList;
			buildConfigurations = (
				D10000000000000000000001 /* Debug */,
				D10000000000000000000002 /* Release */,
			);
			defaultConfigurationIsVisible = 0;
			defaultConfigurationName = Release;
		}};
/* End XCConfigurationList section */
	}};
	rootObject = P10000000000000000000001 /* Project object */;
}}
"#
    );
    let xcode_dir = root.join("YorkApp.xcodeproj");
    std::fs::create_dir_all(&xcode_dir).context("xcodeproj dir")?;
    std::fs::write(xcode_dir.join("project.pbxproj"), pbx).context("write pbxproj")?;
    Ok(())
}

fn sanitize_bundle(name: &str) -> String {
    let mut s: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .map(|c| if c.is_ascii_uppercase() { c.to_ascii_lowercase() } else { c })
        .collect();
    if s.is_empty() {
        s = "yorkapp".into();
    }
    format!("york.{s}.app")
}