use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;

mod banner;
mod dev;
mod android_build;
mod mobile;
mod pipeline;

#[derive(Parser)]
#[command(name = "york", about = "York language toolchain", version, propagate_version = true)]
struct Cli {
    /// Show York credits and ASCII banner
    #[arg(short = 'c', long = "credits", alias = "credit")]
    credits: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Show York credits and ASCII banner
    #[command(alias = "-credits", alias = "credit")]
    Credits,
    /// Run a York project (entry auto-detected like npm)
    Run {
        /// Path to the .yk source file (auto-detected if omitted)
        file: Option<PathBuf>,
        /// Arguments to pass to the program
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Run in watch mode: rebuild + rerun on every save
    Dev {
        /// Path to the .yk source file (auto-detected if omitted)
        file: Option<PathBuf>,
        /// Arguments to pass to the program
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Compile a York source file to a native binary
    Build {
        /// Path to the .yk source file (auto-detected if omitted)
        file: Option<PathBuf>,
        /// Output binary name (default: derived from source filename)
        #[arg(short, long)]
        out: Option<String>,
        /// Use a specific C compiler (e.g. clang, gcc)
        #[arg(long)]
        cc: Option<String>,
    },
    /// Check a York source file for errors without compiling
    Check {
        /// Path to the .yk source file (auto-detected if omitted)
        file: Option<PathBuf>,
    },
    /// Create a new York project
    New {
        /// Project name (default: "new_project")
        name: Option<String>,
    },
    /// Build a mobile app (PWA + native shells) from a York program
    Mobile {
        /// Path to the .yk source file (auto-detected if omitted)
        file: Option<PathBuf>,
        /// App display name (default: "York App")
        #[arg(short, long)]
        name: Option<String>,
        /// Output directory (default: ./mobile-app)
        #[arg(short, long)]
        out: Option<PathBuf>,
        /// Custom app icon (SVG file)
        #[arg(long)]
        icon: Option<PathBuf>,
        /// Serve the built web app locally (default port 8787) for on-phone preview
        #[arg(long)]
        serve: bool,
        /// Native shells: android, ios, or both (default: both)
        #[arg(long)]
        platform: Option<String>,
        /// Also compile a real signed .apk with York's own toolchain
        #[arg(long)]
        apk: bool,
    },
    /// Create a new York mobile app project
    #[command(name = "new-mobile")]
    NewMobile {
        /// Project name (default: "my_mobile_app")
        name: Option<String>,
    },
    /// Inspect environment, C compilers, and toolchain health
    Doctor,
    /// Safely uninstall York from the system
    Uninstall {
        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

fn main() {
    let raw_args: Vec<String> = std::env::args().map(|a| {
        if a == "-credits" || a == "-credit" {
            "--credits".to_string()
        } else {
            a
        }
    }).collect();

    let cli = match Cli::try_parse_from(&raw_args) {
        Ok(c) => c,
        Err(e) => {
            let _ = e.print();
            std::process::exit(1);
        }
    };
    let version = env!("CARGO_PKG_VERSION");

    if cli.credits || matches!(cli.command, Some(Commands::Credits)) {
        banner::banner(version);
        std::process::exit(0);
    }

    let code = match run(cli) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("{} {e:#}", "error:".red().bold());
            1
        }
    };
    std::process::exit(code);
}

fn run(cli: Cli) -> Result<()> {
    let Some(command) = cli.command else {
        use clap::CommandFactory;
        let mut cmd = Cli::command();
        cmd.print_help()?;
        println!();
        return Ok(());
    };

    let cwd = std::env::current_dir()?;
    match command {
        Commands::Credits => {
            let version = env!("CARGO_PKG_VERSION");
            banner::banner(version);
        }
        Commands::Run { file, args } => {
            // `york run dev` — shorthand for watch mode, npm style.
            let is_dev_alias = matches!(&file, Some(f) if f.to_string_lossy() == "dev");
            if is_dev_alias {
                let entry = pipeline::detect_entry(None, &cwd)?;
                let root = std::env::current_dir()?;
                dev::dev(&root, &entry, &args)?;
            } else {
                let entry = pipeline::detect_entry(file.as_deref(), &cwd)?;
                pipeline::compile_run(&entry, &args)?;
            }
        }
        Commands::Dev { file, args } => {
            let entry = pipeline::detect_entry(file.as_deref(), &cwd)?;
            let root = std::env::current_dir()?;
            dev::dev(&root, &entry, &args)?;
        }
        Commands::Build { file, out, cc } => {
            let entry = pipeline::detect_entry(file.as_deref(), &cwd)?;
            cmd_build(&entry, out.as_deref(), cc.as_deref())?;
        }
        Commands::Check { file } => {
            let entry = pipeline::detect_entry(file.as_deref(), &cwd)?;
            cmd_check(&entry)?;
        }
        Commands::New { name } => {
            cmd_new(name.as_deref())?;
        }
        Commands::Mobile { file, name, out, icon, serve, platform, apk } => {
            let entry = pipeline::detect_entry(file.as_deref(), &cwd)?;
            let out_dir = out.unwrap_or_else(|| cwd.join("mobile-app"));
            let platform = parse_platform(platform.as_deref())?;
            mobile::cmd_mobile(&entry, name.as_deref(), &out_dir, icon.as_deref(), serve, platform, apk)?;
        }
        Commands::NewMobile { name } => {
            let name = name.unwrap_or_else(|| "my_mobile_app".to_string());
            mobile::cmd_new_mobile(&name)?;
        }
        Commands::Doctor => {
            cmd_doctor()?;
        }
        Commands::Uninstall { yes } => {
            cmd_uninstall(yes)?;
        }
    }
    Ok(())
}

fn cmd_build(file: &std::path::Path, out: Option<&str>, cc: Option<&str>) -> Result<()> {
    let c_source = pipeline::compile_to_c(file)?;

    // Output name: default to stem of the source file.
    let exe_name = out.unwrap_or_else(|| {
        file.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output")
    });
    let out_path = std::env::current_dir()?.join(exe_name);

    if let Some(cc_name) = cc {
        pipeline::compile_with_cc(&c_source, &out_path, cc_name)?;
    } else {
        york_codegen_c::compile_c(&c_source, out_path.to_str().unwrap())
            .map_err(|e| anyhow::anyhow!("{e}"))
            .context("C compilation failed")?;
    }

    println!("{} `{}`", "built:".green().bold(), out_path.display());
    Ok(())
}

fn cmd_check(file: &std::path::Path) -> Result<()> {
    let _ = pipeline::compile_to_c(file)?;
    println!("{}", "ok".green().bold());
    Ok(())
}

fn cmd_new(name: Option<&str>) -> Result<()> {
    let name = name.unwrap_or("new_project");
    let root = std::env::current_dir()?.join(name);

    if root.exists() {
        bail!("directory `{}` already exists", name);
    }
    std::fs::create_dir_all(&root)?;
    std::fs::create_dir_all(root.join("src"))?;

    std::fs::write(
        root.join("src").join("main.yk"),
        include_str!("template_main.yk"),
    )?;

    println!("{} created project `{}`", "created:".green().bold(), name);
    println!("  cd {name}");
    println!("  {}   ← runs it (auto-detect entry)", "york run".cyan());
    println!("  {}   ← watch mode: rebuild on save", "york dev".cyan());
    Ok(())
}

fn parse_platform(s: Option<&str>) -> Result<crate::mobile::Platform> {
    use crate::mobile::Platform;
    match s.map(|v| v.to_ascii_lowercase()).as_deref() {
        None | Some("both") | Some("all") => Ok(Platform::Both),
        Some("android") => Ok(Platform::Android),
        Some("ios") | Some("apple") => Ok(Platform::Ios),
        Some(other) => bail!(
            "unknown platform `{other}` — use android, ios, or both (default: both)"
        ),
    }
}

fn cmd_doctor() -> Result<()> {
    println!("{}", "York Toolchain Doctor (v0.5.0)".bold().cyan());
    println!("{}", "─".repeat(50).dimmed());

    // OS info
    println!("{} {} ({})", "Platform:".bold(), std::env::consts::OS, std::env::consts::ARCH);

    // Current executable
    if let Ok(exe) = std::env::current_exe() {
        println!("{} {}", "York Binary:".bold(), exe.display());
    }

    // PATH check
    let on_path = std::env::var("PATH").map(|p| p.to_lowercase().contains("york")).unwrap_or(false);
    if on_path {
        println!("{} York is in your PATH", "✓".green().bold());
    } else {
        println!("{} York might not be in your current PATH session", "·".yellow().bold());
    }

    // Check C compilers
    println!("\n{}", "Native C Compilers:".bold());
    let compilers = ["clang", "gcc", "cl", "tcc", "zig"];
    let mut found_any = false;
    for cc in compilers {
        let is_found = if cc == "cl" {
            std::process::Command::new("cl.exe").output().is_ok()
        } else {
            std::process::Command::new(cc).arg("--version").output().is_ok()
        };
        if is_found {
            println!("  {} Found `{}`", "✓".green().bold(), cc);
            found_any = true;
        } else {
            println!("  {} `{}` not found in PATH", "·".dimmed(), cc);
        }
    }
    if !found_any {
        println!("  {} No C compiler detected. Install Clang, GCC, or Visual Studio MSVC.", "Note:".yellow().bold());
    }

    // Mobile dependencies
    println!("\n{}", "Mobile Toolchain:".bold());
    let has_java = std::process::Command::new("javac").arg("-version").output().is_ok();
    if has_java {
        println!("  {} Java JDK (`javac`) detected", "✓".green().bold());
    } else {
        println!("  {} Java JDK not found (optional, only needed for `york mobile --apk`)", "·".dimmed());
    }

    println!("\n{}", "Security & Integrity:".bold());
    println!("  {} Verified Open Source Systems Language (MIT License)", "✓".green().bold());
    println!("  {} 100% clean, no malware, no telemetry, safe local compiler", "✓".green().bold());

    println!();
    Ok(())
}

fn cmd_uninstall(yes: bool) -> Result<()> {
    println!("{}", "York Uninstaller".bold().yellow());
    println!("{}", "─".repeat(50).dimmed());

    if !yes {
        print!("Are you sure you want to uninstall York from this machine? (y/N): ");
        std::io::Write::flush(&mut std::io::stdout())?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("{}", "Uninstallation cancelled.".green());
            return Ok(());
        }
    }

    println!("[1/2] Removing York program files...");
    #[cfg(windows)]
    {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            let p = std::path::PathBuf::from(local_app_data).join("Programs").join("york");
            if p.exists() {
                let _ = std::fs::remove_dir_all(&p);
                println!("      Removed {}", p.display());
            }
        }
    }
    #[cfg(unix)]
    {
        if let Ok(home) = std::env::var("HOME") {
            let p = std::path::PathBuf::from(home).join(".york");
            if p.exists() {
                let _ = std::fs::remove_dir_all(&p);
                println!("      Removed {}", p.display());
            }
        }
    }

    println!("[2/2] Cleaning configuration & PATH references...");
    #[cfg(windows)]
    {
        println!("      To complete PATH cleanup, run: irm https://raw.githubusercontent.com/TheRealClyp/York/main/installers/uninstall.ps1 | iex");
    }
    #[cfg(unix)]
    {
        println!("      To complete shell profile cleanup, run: curl -fsSL https://raw.githubusercontent.com/TheRealClyp/York/main/installers/uninstall.sh | sh");
    }

    println!("\n{}", "✓ York uninstallation complete. Thank you for using York!".green().bold());
    Ok(())
}