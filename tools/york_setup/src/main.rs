// york-setup — Windows GUI installer for the York language, node style.
// Pure Win32, no external toolchains. Shows a setup window with live
// progress, downloads the release binary with a real progress bar, and
// falls back to an embedded copy when offline.
//
//   york-setup.exe             install York (default: %LOCALAPPDATA%\Programs\york)
//   york-setup.exe --dir=...   install to a custom directory
//   york-setup.exe --no-path   install without touching PATH
//   york-setup.exe --uninstall remove York and clean PATH

#![cfg(windows)]
#![windows_subsystem = "windows"]

use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi as gdi;
use windows_sys::Win32::Networking::WinInet as net;
use windows_sys::Win32::System::Threading as thr;
use windows_sys::Win32::UI::Controls as ctl;
use windows_sys::Win32::UI::WindowsAndMessaging as wm;

const VERSION: &str = "0.5.0";
const DL_URL: &str =
    "https://github.com/TheRealClyp/York/releases/download/v0.5.0/york-x86_64-windows.exe";
const BIN: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../target/release/york.exe"
));

const WM_ENABLE: u32 = 0x000A;

const IDC_HDR: i32 = 100;
const IDC_PROGRESS: i32 = 101;
const IDC_STATUS: i32 = 102;
const IDC_FOOTER: i32 = 103;
const IDC_DONE: i32 = 104;
const IDC_BANNER_FIRST: i32 = 200;

const YORK_ART: [&str; 6] = [
    "   ██╗  ██╗ ██████╗ ██████╗ ██╗  ██╗",
    "   ██║ ██╔╝██╔═══██╗██╔══██╗██║ ██╔╝",
    "   █████╔╝ ██║   ██║██████╔╝█████╔╝ ",
    "   ██╔═██╗ ██║   ██║██╔══██╗██╔═██╗ ",
    "   ██║  ██╗╚██████╔╝██║  ██║██║  ██╗",
    "   ╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═╝╚═╝  ╚═╝",
];
const WASAY_ART: [&str; 6] = [
    "██╗    ██╗  █████╗  ███████╗  █████╗  ██╗   ██╗",
    "██║    ██║  ██╔══██╗  ██╔════╝  ██╔══██╗  ╚██╗ ██╔╝",
    "██║ █╗ ██║  ███████║  ███████╗  ███████║   ╚████╔╝ ",
    "██║███╗██║  ██╔══██║  ╚════██║  ██╔══██║    ╚██╔╝  ",
    "╚███╔███╔╝  ██║  ██║  ███████║  ██║  ██║     ██║   ",
    " ╚══╝╚══╝  ╚═╝  ╚═╝  ╚══════╝  ╚═╝  ╚═╝     ╚═╝   ",
];

const COLOR_STR: [u32; 12] = [
    0x67E8F9, 0x67E8F9, 0xA78BFA, 0xA78BFA, 0x67E8F9, 0x67E8F9, // YORK
    0xA78BFA, 0xA78BFA, 0xA78BFA, 0xA78BFA, 0xA78BFA, 0xA78BFA, // WASAY
];

fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16)
}

fn w(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn mb(n: u64) -> String {
    format!("{:.1} MB", n as f64 / 1_000_000.0)
}

struct Handles {
    prog: HWND,
    status: HWND,
    done: HWND,
}
unsafe impl Send for Handles {}
unsafe impl Sync for Handles {}

static H: OnceLock<Handles> = OnceLock::new();
static STOP: AtomicBool = AtomicBool::new(false);

struct Brushes(gdi::HBRUSH, gdi::HBRUSH, gdi::HBRUSH, gdi::HBRUSH);
unsafe impl Send for Brushes {}
unsafe impl Sync for Brushes {}
static BRUSHES: OnceLock<Brushes> = OnceLock::new();

fn set_status(text: &str) {
    if let Some(h) = H.get() {
        let v = w(text);
        unsafe {
            wm::SetWindowTextW(h.status, v.as_ptr());
        }
    }
}

fn set_progress(p: i32) {
    if let Some(h) = H.get() {
        unsafe {
            wm::SendMessageW(h.prog, ctl::PBM_SETPOS, p as usize, 0);
        }
    }
}

fn set_done_enabled(enabled: bool) {
    if let Some(h) = H.get() {
        unsafe {
            wm::SendMessageW(h.done, WM_ENABLE, if enabled { 1 } else { 0 }, 0);
        }
    }
}

fn lerp(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t) as u8
}

unsafe fn paint_gradient(hdc: gdi::HDC, r: &RECT, c1: COLORREF, c2: COLORREF) {
    let width = (r.right - r.left) as i64;
    let a = [(c1 >> 0) as u8, (c1 >> 8) as u8, (c1 >> 16) as u8];
    let b = [(c2 >> 0) as u8, (c2 >> 8) as u8, (c2 >> 16) as u8];
    for i in 0..96 {
        let t = i as f32 / 95.0;
        let col = rgb(lerp(a[0], b[0], t), lerp(a[1], b[1], t), lerp(a[2], b[2], t));
        let brush = gdi::CreateSolidBrush(col);
        let x1 = r.left + ((width * i as i64) / 96) as i32;
        let x2 = r.left + ((width * (i as i64 + 1)) / 96) as i32;
        let rr = RECT { left: x1, top: r.top, right: x2, bottom: r.bottom };
        gdi::FillRect(hdc, &rr, brush);
        gdi::DeleteObject(brush);
    }
}

unsafe extern "system" fn header_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == wm::WM_PAINT {
        let mut ps = std::mem::zeroed::<gdi::PAINTSTRUCT>();
        let hdc = gdi::BeginPaint(hwnd, &mut ps);
        let mut r = std::mem::zeroed::<RECT>();
        wm::GetClientRect(hwnd, &mut r);
        paint_gradient(hdc, &r, rgb(0x0E, 0x22, 0x3C), rgb(0x17, 0x3F, 0x6B));

        gdi::SetBkMode(hdc, gdi::TRANSPARENT as i32);
        let word = w("York");
        let tag = w(&format!("Setup v{VERSION}"));

        let title_font = gdi::CreateFontW(-38, 0, 0, 0, 700, 0, 0, 0, 1, 0, 0, 5, 0, w("Segoe UI").as_ptr());
        let old = gdi::SelectObject(hdc, title_font);
        gdi::SetTextColor(hdc, rgb(0xF2, 0xF7, 0xFB));
        gdi::TextOutW(hdc, 26, 16, word.as_ptr(), word.len() as i32);
        gdi::SelectObject(hdc, old);

        let tag_font = gdi::CreateFontW(-17, 0, 0, 0, 600, 0, 0, 0, 1, 0, 0, 5, 0, w("Segoe UI").as_ptr());
        let old = gdi::SelectObject(hdc, tag_font);
        gdi::SetTextColor(hdc, rgb(0x67, 0xE8, 0xF9));
        gdi::TextOutW(hdc, 28, 62, tag.as_ptr(), tag.len() as i32);
        gdi::SelectObject(hdc, old);

        gdi::DeleteObject(title_font);
        gdi::DeleteObject(tag_font);
        gdi::EndPaint(hwnd, &ps);
        return 0;
    }
    wm::DefWindowProcW(hwnd, msg, wp, lp)
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        wm::WM_COMMAND => {
            let id = (wparam & 0xFFFF) as u32;
            if id == IDC_DONE as u32 {
                wm::SendMessageW(hwnd, wm::WM_CLOSE, 0, 0);
                return 0;
            }
            wm::DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        wm::WM_CTLCOLORSTATIC => {
            let ctrl = wm::GetDlgCtrlID(lparam as HWND);
            let text_color = if (IDC_BANNER_FIRST..IDC_BANNER_FIRST + 12).contains(&ctrl) {
                COLOR_STR[(ctrl - IDC_BANNER_FIRST) as usize]
            } else if ctrl == IDC_FOOTER {
                rgb(0x8A, 0xA0, 0xB8)
            } else {
                rgb(0xE6, 0xED, 0xF3)
            };
            gdi::SetTextColor(wparam as gdi::HDC, text_color);
            gdi::SetBkMode(wparam as gdi::HDC, gdi::TRANSPARENT as i32);
            BRUSHES.get().unwrap().3 as LRESULT
        }
        wm::WM_ERASEBKGND => {
            let hdc = wparam as gdi::HDC;
            let mut r = std::mem::zeroed::<RECT>();
            wm::GetClientRect(hwnd, &mut r);
            paint_gradient(hdc, &r, rgb(0x0B, 0x0F, 0x1A), rgb(0x10, 0x21, 0x3A));
            1
        }
        wm::WM_CLOSE => {
            STOP.store(true, Ordering::Relaxed);
            wm::DestroyWindow(hwnd);
            0
        }
        wm::WM_DESTROY => {
            wm::PostQuitMessage(0);
            0
        }
        _ => wm::DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn register_class(hinst: HINSTANCE, name: windows_sys::core::PCWSTR, proc: wm::WNDPROC) {
    unsafe {
        let mut wc: wm::WNDCLASSW = std::mem::zeroed();
        wc.lpfnWndProc = proc;
        wc.hInstance = hinst;
        wc.hbrBackground = std::ptr::null_mut();
        wc.lpszClassName = name;
        wm::RegisterClassW(&wc);
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn create_static(
    parent: HWND,
    id: i32,
    text: &str,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    hinst: HINSTANCE,
    font: gdi::HFONT,
) -> HWND {
    let v = w(text);
    let hwnd = wm::CreateWindowExW(
        0,
        w("STATIC").as_ptr(),
        v.as_ptr(),
        wm::WS_CHILD | wm::WS_VISIBLE,
        x,
        y,
        width,
        height,
        parent,
        id as wm::HMENU,
        hinst,
        std::ptr::null(),
    );
    wm::SendMessageW(hwnd, wm::WM_SETFONT, font as usize, 1);
    hwnd
}

fn install_dir(override_dir: Option<&str>) -> PathBuf {
    match override_dir {
        Some(d) => PathBuf::from(d),
        None => {
            let root = env::var("LOCALAPPDATA")
                .or_else(|_| env::var("USERPROFILE").map(|p| format!(r"{p}\AppData\Local")))
                .unwrap_or_else(|_| ".".into());
            PathBuf::from(root).join(r"Programs\york")
        }
    }
}

unsafe fn download(url: &str, target: &Path) -> Result<u64, String> {
    let agent = w("york-setup/0.1.0");
    let h = net::InternetOpenW(agent.as_ptr(), net::INTERNET_OPEN_TYPE_PRECONFIG, std::ptr::null(), std::ptr::null(), 0);
    if h.is_null() {
        return Err("network init failed".into());
    }
    let urlw = w(url);
    let flags = 0x8000_0000u32 | 0x0400_0000u32 | net::INTERNET_FLAG_SECURE;
    let req = net::InternetOpenUrlW(h, urlw.as_ptr(), std::ptr::null(), 0, flags, 0);
    if req.is_null() {
        net::InternetCloseHandle(h);
        return Err("request failed".into());
    }
    let mut status_buf = [0u8; 16];
    let mut status_len = 16u32;
    let qs = net::HttpQueryInfoW(
        req,
        net::HTTP_QUERY_STATUS_CODE,
        status_buf.as_mut_ptr() as _,
        &mut status_len,
        std::ptr::null_mut(),
    );
    if qs != 0 {
        let code: u32 = String::from_utf8_lossy(&status_buf[..status_len as usize])
            .trim()
            .parse()
            .unwrap_or(0);
        if code != 200 {
            net::InternetCloseHandle(req);
            net::InternetCloseHandle(h);
            return Err(format!("HTTP {code}"));
        }
    }
    let mut total: u64 = 0;
    let mut clen_bytes = [0u8; 32];
    let mut clen = 32u32;
    let ok = net::HttpQueryInfoW(req, net::HTTP_QUERY_CONTENT_LENGTH, clen_bytes.as_mut_ptr() as _, &mut clen, std::ptr::null_mut());
    let expected: u64 = if ok != 0 {
        String::from_utf8_lossy(&clen_bytes[..clen as usize]).trim().parse().unwrap_or(0)
    } else {
        0
    };
    fs::create_dir_all(target.parent().unwrap_or(Path::new("."))).unwrap_or(());
    let mut f = fs::File::create(target).map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        if STOP.load(Ordering::Relaxed) {
            net::InternetCloseHandle(req);
            net::InternetCloseHandle(h);
            return Err("cancelled".into());
        }
        let mut read = 0u32;
        let r = net::InternetReadFile(req, buf.as_mut_ptr() as _, buf.len() as u32, &mut read);
        if r == 0 || read == 0 {
            break;
        }
        f.write_all(&buf[..read as usize]).map_err(|e| e.to_string())?;
        total += read as u64;
        if expected > 0 {
            set_progress(((total as f64 / expected as f64) * 45.0) as i32);
            set_status(&format!(
                "Downloading york.exe — {}% ({} / {})",
                total * 100 / expected,
                mb(total),
                mb(expected)
            ));
        }
    }
    net::InternetCloseHandle(req);
    net::InternetCloseHandle(h);
    Ok(total)
}

fn install_targets(override_dir: Option<&str>) -> (PathBuf, PathBuf) {
    let dir = install_dir(override_dir);
    let exe = dir.join(r"bin\york.exe");
    (dir, exe)
}

fn add_to_path(bin_dir: &Path) {
    let script = format!(
        "if ('{}' -notin [Environment]::GetEnvironmentVariable('Path','User').Split(';')) {{ [Environment]::SetEnvironmentVariable('Path', ([Environment]::GetEnvironmentVariable('Path','User').TrimEnd(';') + ';{}'), 'User') }}",
        bin_dir.to_string_lossy(),
        bin_dir.to_string_lossy(),
    );
    let _ = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .status();
}

fn remove_from_path(bin_dir: &Path) {
    let script = format!(
        "[Environment]::SetEnvironmentVariable('Path', (([Environment]::GetEnvironmentVariable('Path','User').Split(';') | Where-Object {{ $_ -ne '{}' }}) -join ';'), 'User')",
        bin_dir.to_string_lossy(),
    );
    let _ = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .status();
}

fn verify(exe: &Path) -> bool {
    Command::new(exe)
        .arg("--version")
        .output()
        .map(|o| o.status.success() && !o.stdout.is_empty())
        .unwrap_or(false)
}

unsafe fn worker_install(_stop: &AtomicBool, dir: &Path, exe: &Path, no_path: bool) {
    set_status("Preparing install directory…");
    set_progress(0);
    if let Err(e) = fs::create_dir_all(exe.parent().unwrap()) {
        set_status(&format!("Could not create install directory: {e}"));
        set_done_enabled(true);
        return;
    }

    // Step 1 — download with a live progress bar; fall back to bundled copy.
    match download(DL_URL, exe) {
        Ok(n) if n > 0 => {
            if verify(exe) {
                set_status("Download complete — installing…");
            } else {
                set_status("Downloaded file failed to verify — using the bundled copy.");
                set_progress(48);
                fs::write(exe, BIN).unwrap_or(());
            }
        }
        _ => {
            set_status("Offline — using the bundled copy.");
            set_progress(48);
            fs::write(exe, BIN).unwrap_or(());
        }
    }

    // Step 2 — finalize the binary.
    set_status("Installing york.exe…");
    set_progress(58);
    if !exe.exists() {
        fs::write(exe, BIN).unwrap_or(());
    }

    // Step 3 — PATH.
    set_status("Adding york to your PATH…");
    set_progress(68);
    if !no_path {
        add_to_path(exe.parent().unwrap());
    }

    // Step 4 — verify.
    set_status("Verifying the install…");
    set_progress(78);
    if verify(exe) {
        set_status("York 0.1.0 installed — `york` is ready to go.");
    } else {
        set_status("York installed, but verification failed. Restart your terminal and try `york --version`.");
    }
    set_progress(100);
    set_done_enabled(true);
    let _ = dir;
}

unsafe fn worker_uninstall(_stop: &AtomicBool, dir: &Path, _exe: &Path, no_path: bool) {
    set_status("Removing York…");
    set_progress(20);
    if fs::remove_dir_all(dir).is_ok() {
        set_progress(55);
    }
    if !no_path {
        remove_from_path(&dir.join(r"bin"));
        set_progress(80);
    }
    set_status("York is uninstalled. See you around!");
    set_progress(100);
    set_done_enabled(true);
}

type WorkerFn = unsafe fn(stop: &AtomicBool, dir: &Path, exe: &Path, no_path: bool);

fn main() {
    unsafe {
        let hinst = windows_sys::Win32::System::LibraryLoader::GetModuleHandleW(std::ptr::null());

        let args: Vec<String> = env::args().skip(1).collect();
        let mut dir: Option<String> = None;
        let mut no_path = false;
        let mut uninstall = false;
        for a in &args {
            if a == "--uninstall" {
                uninstall = true;
            } else if a == "--no-path" {
                no_path = true;
            } else if let Some(v) = a.strip_prefix("--dir=") {
                dir = Some(v.to_string());
            }
        }
        let (dir, exe) = install_targets(dir.as_deref());

        let cc = ctl::INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<ctl::INITCOMMONCONTROLSEX>() as u32,
            dwICC: ctl::ICC_PROGRESS_CLASS,
        };
        ctl::InitCommonControlsEx(&cc);

        let class_name = windows_sys::core::w!("YorkSetupWnd");
        let header_class = windows_sys::core::w!("YorkSetupHeader");
        register_class(hinst, class_name, Some(wnd_proc));
        register_class(hinst, header_class, Some(header_proc));

        let _ = BRUSHES.set(Brushes(
            gdi::CreateSolidBrush(rgb(0x67, 0xE8, 0xF9)),
            gdi::CreateSolidBrush(rgb(0xA7, 0x8B, 0xFA)),
            gdi::CreateSolidBrush(rgb(0x10, 0x21, 0x3A)),
            gdi::CreateSolidBrush(rgb(0x0B, 0x0F, 0x1A)),
        ));

        let title = w(if uninstall { "York Setup — Uninstall" } else { "York Setup" });
        let main_hwnd = wm::CreateWindowExW(
            0,
            class_name,
            title.as_ptr(),
            wm::WS_CAPTION | wm::WS_SYSMENU | wm::WS_MINIMIZEBOX,
            wm::CW_USEDEFAULT,
            wm::CW_USEDEFAULT,
            520,
            470,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            hinst,
            std::ptr::null(),
        );

        if main_hwnd.is_null() {
            return;
        }

        let _header = wm::CreateWindowExW(
            0,
            header_class,
            std::ptr::null(),
            wm::WS_CHILD | wm::WS_VISIBLE,
            0,
            0,
            520,
            100,
            main_hwnd,
            IDC_HDR as wm::HMENU,
            hinst,
            std::ptr::null(),
        );

        // Banner art — 12 monospace lines, colored per const table.
        let banner_font = gdi::CreateFontW(-13, 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 5, 0x300, w("Cascadia Code").as_ptr());
        let mut y = 112;
        for (i, line) in YORK_ART.iter().chain(WASAY_ART.iter()).enumerate() {
            create_static(main_hwnd, IDC_BANNER_FIRST + i as i32, line, 33, y, 460, 16, hinst, banner_font);
            y += 16;
        }

        // Progress bar.
        let prog = wm::CreateWindowExW(
            0,
            ctl::PROGRESS_CLASSW,
            std::ptr::null(),
            wm::WS_CHILD | wm::WS_VISIBLE,
            34,
            310,
            452,
            14,
            main_hwnd,
            IDC_PROGRESS as wm::HMENU,
            hinst,
            std::ptr::null(),
        );
        wm::SendMessageW(prog, ctl::PBM_SETRANGE32, 0, 100);

        // Status + footer labels.
        let status_font = gdi::CreateFontW(-16, 0, 0, 0, 500, 0, 0, 0, 1, 0, 0, 5, 0, w("Segoe UI").as_ptr());
        let status = create_static(main_hwnd, IDC_STATUS, "", 36, 338, 452, 22, hinst, status_font);

        let footer_font = gdi::CreateFontW(-14, 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 5, 0, w("Segoe UI").as_ptr());
        let _footer = create_static(main_hwnd, IDC_FOOTER, "York 0.5.0 · MIT · github.com/TheRealClyp/York", 36, 372, 452, 20, hinst, footer_font);

        // Done button.
        let done = wm::CreateWindowExW(
            0,
            w("BUTTON").as_ptr(),
            w("Done").as_ptr(),
            wm::WS_CHILD | wm::WS_VISIBLE | 0x0000_0001u32, // BS_DEFPUSHBUTTON
            420,
            372,
            64,
            30,
            main_hwnd,
            IDC_DONE as wm::HMENU,
            hinst,
            std::ptr::null(),
        );
        wm::SendMessageW(done, WM_ENABLE, 0, 0);

        let _ = H.set(Handles { prog, status, done });

        // Kick off the worker.
        let worker: WorkerFn = if uninstall { worker_uninstall } else { worker_install };
        let payload = InstallArgs { dir: dir.clone(), exe: exe.clone(), no_path, worker };
        let pbox: Box<InstallArgs> = Box::new(payload);
        thr::CreateThread(
            std::ptr::null(),
            0,
            Some(thread_entry),
            Box::into_raw(pbox) as *const core::ffi::c_void,
            0,
            std::ptr::null_mut(),
        );

        wm::ShowWindow(main_hwnd, wm::SW_NORMAL);

        let mut msg = std::mem::zeroed::<wm::MSG>();
        while wm::GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            wm::TranslateMessage(&msg);
            wm::DispatchMessageW(&msg);
        }
    }
}

struct InstallArgs {
    dir: PathBuf,
    exe: PathBuf,
    no_path: bool,
    worker: WorkerFn,
}

unsafe extern "system" fn thread_entry(param: *mut core::ffi::c_void) -> u32 {
    let p = param as *mut InstallArgs;
    let arc = unsafe { Box::from_raw(p) };
    let (worker, dir, exe, no_path) = (arc.worker, arc.dir.clone(), arc.exe.clone(), arc.no_path);
    unsafe {
        worker(&STOP, &dir, &exe, no_path);
    }
    0
}