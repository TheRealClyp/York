use clap::{Parser, ArgAction};
use colored::*;
use std::path::PathBuf;
use std::process::Command;
use std::fs;
use anyhow::{Context, Result};

#[derive(Parser, Debug)]
#[command(name = "yc", version = "1.1.0", about = "yc — The Official York Systems Compiler (GCC/Clang-compatible driver)", long_about = None)]
struct Cli {
    /// Input source files (.yk or .c)
    #[arg(required = true)]
    inputs: Vec<PathBuf>,

    /// Output executable filename
    #[arg(short = 'o', long = "output", default_value = "a.out")]
    output: PathBuf,

    /// Optimization level (-O0, -O1, -O2, -O3)
    #[arg(short = 'O', default_value = "2")]
    opt_level: String,

    /// Only run lex, parse, semantic analysis and C codegen (do not invoke system compiler)
    #[arg(short = 'c', action = ArgAction::SetTrue)]
    compile_only: bool,

    /// Verbose compiler output
    #[arg(short = 'v', action = ArgAction::SetTrue)]
    verbose: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let input = &cli.inputs[0];
    if !input.exists() {
        anyhow::bail!("{}: error: input file '{}' does not exist", "yc".red().bold(), input.display());
    }

    println!("{} {} compiling {} -> {}", "yc".cyan().bold(), format!("(v1.1.0)").dimmed(), input.display(), cli.output.display());

    let ext = input.extension().and_then(|e| e.to_str()).unwrap_or("");
    let c_code = if ext == "yk" {
        let source = fs::read_to_string(input)
            .with_context(|| format!("failed to read source file `{}`", input.display()))?;

        if cli.verbose {
            println!("  {} lexical analysis & parsing...", "phase 1/3".yellow());
        }
        let lexed = york_lexer::lex(&source);
        if !lexed.errors.is_empty() {
            for e in &lexed.errors {
                eprintln!("{} {e}", "error:".red().bold());
            }
            anyhow::bail!("lexer failed with {} error(s)", lexed.errors.len());
        }

        let parsed = york_parser::parse(&lexed.tokens);
        if !parsed.errors.is_empty() {
            for e in &parsed.errors {
                eprintln!("{} {e}", "error:".red().bold());
            }
            anyhow::bail!("parser failed with {} error(s)", parsed.errors.len());
        }

        if cli.verbose {
            println!("  {} semantic analysis & typed IR lowering...", "phase 2/3".yellow());
        }
        let sema = york_sema::analyze(&parsed.program);
        if !sema.errors.is_empty() {
            for e in &sema.errors {
                eprintln!("{} {e}", "error:".red().bold());
            }
            anyhow::bail!("semantic analysis failed with {} error(s)", sema.errors.len());
        }

        if cli.verbose {
            println!("  {} generating optimized C11 code...", "phase 3/3".yellow());
        }
        york_codegen_c::generate(&sema.program)
    } else if ext == "c" {
        fs::read_to_string(input)?
    } else {
        anyhow::bail!("yc: error: unsupported input file extension `.{}` (expected .yk or .c)", ext);
    };

    let temp_c = input.with_extension("tmp.c");
    fs::write(&temp_c, &c_code)?;

    if cli.compile_only {
        println!("{} compilation successful (emitted {})", "yc:".green().bold(), temp_c.display());
        return Ok(());
    }

    let opt_flag = format!("-O{}", cli.opt_level);
    let out_str = cli.output.to_string_lossy().to_string();

    let cc = detect_compiler();
    if cli.verbose {
        println!("  {} invoking backend driver `{}` with `{}`...", "linking".yellow(), cc, opt_flag);
    }

    let status = match cc.as_str() {
        "cl" => {
            Command::new("cl.exe")
                .arg(&temp_c)
                .arg(format!("/Fe:{}", out_str))
                .arg(format!("/O{}", &cli.opt_level))
                .arg("user32.lib")
                .arg("gdi32.lib")
                .arg("shell32.lib")
                .arg("ws2_32.lib")
                .arg("advapi32.lib")
                .status()
        }
        _ => {
            let mut cmd = Command::new(&cc);
            cmd.arg(&temp_c)
               .arg("-o")
               .arg(&out_str)
               .arg(&opt_flag)
               .arg("-lm");
            
            #[cfg(target_os = "windows")]
            {
                cmd.arg("-luser32").arg("-lgdi32").arg("-lshell32").arg("-lws2_32").arg("-ladvapi32");
            }
            cmd.status()
        }
    };

    let _ = fs::remove_file(&temp_c);

    match status {
        Ok(s) if s.success() => {
            println!("{} build complete! Output -> {}", "SUCCESS".green().bold(), cli.output.display());
            Ok(())
        }
        Ok(s) => {
            anyhow::bail!("yc: error: backend compiler exited with code {}", s.code().unwrap_or(-1));
        }
        Err(e) => {
            anyhow::bail!("yc: error: failed to execute backend compiler `{}`: {}", cc, e);
        }
    }
}

fn detect_compiler() -> String {
    for cc in &["clang", "gcc", "cc", "tcc", "zig"] {
        if Command::new(cc).arg("--version").status().map(|s| s.success()).unwrap_or(false) {
            if *cc == "zig" {
                return "zig".to_string();
            }
            return cc.to_string();
        }
    }
    if cfg!(target_os = "windows") {
        if Command::new("cl.exe").status().is_ok() {
            return "cl".to_string();
        }
    }
    "cc".to_string()
}
