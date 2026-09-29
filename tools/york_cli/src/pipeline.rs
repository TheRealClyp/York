use std::path::{Path, PathBuf};
use std::process;

use anyhow::{bail, Context, Result};
use colored::Colorize;

/// Compile a .yk file to C source.
pub fn compile_to_c(file: &Path) -> Result<String> {
    let source = std::fs::read_to_string(file)
        .with_context(|| format!("cannot read `{}`", file.display()))?;

    let lexed = york_lexer::lex(&source);
    if !lexed.errors.is_empty() {
        for e in &lexed.errors {
            eprintln!("{} {e}", "error:".red().bold());
        }
        bail!("lexer failed with {} error(s)", lexed.errors.len());
    }
    let parsed = york_parser::parse(&lexed.tokens);
    if !parsed.errors.is_empty() {
        for e in &parsed.errors {
            eprintln!("{} {e}", "error:".red().bold());
        }
        bail!("parser failed with {} error(s)", parsed.errors.len());
    }
    let sema = york_sema::analyze(&parsed.program);
    if !sema.errors.is_empty() {
        for e in &sema.errors {
            eprintln!("{} {e}", "error:".red().bold());
        }
        bail!("semantic analysis failed with {} error(s)", sema.errors.len());
    }
    Ok(york_codegen_c::generate(&sema.program))
}

/// Compile a .yk file, write C to a temp dir, invoke the C compiler,
/// then run the resulting binary from the source file's directory.
/// Returns elapsed compile time on success.
pub fn compile_run(file: &Path, args: &[String]) -> Result<std::time::Duration> {
    let t = std::time::Instant::now();
    let c_source = compile_to_c(file)?;

    let dir = std::env::temp_dir().join(format!("york_run_{}", process::id()));
    std::fs::create_dir_all(&dir).context("failed to create temp directory")?;
    let exe = dir.join("york_out");

    york_codegen_c::compile_c(&c_source, exe.to_str().unwrap())
        .map_err(|e| anyhow::anyhow!("{e}"))
        .context("C compilation failed")?;

    let exe_path = resolve_exe(&exe);
    let run_dir = match file.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => std::path::PathBuf::from("."),
    };
    let status = process::Command::new(&exe_path)
        .current_dir(run_dir)
        .args(args)
        .status()
        .with_context(|| format!("failed to execute `{}`", exe_path.display()))?;

    let elapsed = t.elapsed();
    let _ = std::fs::remove_dir_all(&dir);

    if !status.success() {
        process::exit(status.code().unwrap_or(1));
    }
    Ok(elapsed)
}

/// Detect the entry file like npm does:
/// 1. explicit path given → use it
/// 2. `main.yk` in cwd
/// 3. `src/main.yk` (or src/index.yk)
/// 4. any single `*.yk` file in cwd
pub fn detect_entry(explicit: Option<&Path>, cwd: &Path) -> Result<PathBuf> {
    if let Some(p) = explicit {
        // If explicit path is a manifest file (resource.ypg), read entry from it.
        if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
            if name == "resource.ypg" || p.extension().and_then(|x| x.to_str()) == Some("ypg") {
                let content = std::fs::read_to_string(p).with_context(|| format!("cannot read manifest `{}`", p.display()))?;
                for line in content.lines() {
                    let l = line.trim();
                    if l.starts_with("entry") || l.starts_with("main") {
                        if let Some(start) = l.find('"') {
                            if let Some(end) = l[start + 1..].find('"') {
                                let path_str = &l[start + 1..start + 1 + end];
                                let resolved = p.parent().unwrap_or(cwd).join(path_str);
                                if resolved.is_file() {
                                    return Ok(resolved);
                                }
                            }
                        }
                    }
                }
            }
        }
        return Ok(p.to_path_buf());
    }

    // Check for local project manifest `resource.ypg` in cwd
    let manifest_path = cwd.join("resource.ypg");
    if manifest_path.is_file() {
        let content = std::fs::read_to_string(&manifest_path).unwrap_or_default();
        for line in content.lines() {
            let l = line.trim();
            if l.starts_with("entry") || l.starts_with("main") {
                if let Some(start) = l.find('"') {
                    if let Some(end) = l[start + 1..].find('"') {
                        let path_str = &l[start + 1..start + 1 + end];
                        let resolved = cwd.join(path_str);
                        if resolved.is_file() {
                            return Ok(resolved);
                        }
                    }
                }
            }
        }
    }

    let candidates = [
        cwd.join("main.yk"),
        cwd.join("src").join("main.yk"),
        cwd.join("src").join("index.yk"),
    ];
    for c in &candidates {
        if c.is_file() {
            return Ok(c.clone());
        }
    }

    // Scan all .yk files in cwd
    let mut found = Vec::new();
    if let Ok(rd) = std::fs::read_dir(cwd) {
        for e in rd.flatten() {
            let path = e.path();
            if path.extension().and_then(|x| x.to_str()) == Some("yk") {
                found.push(path);
            }
        }
    }

    if found.len() == 1 {
        return Ok(found.remove(0));
    } else if found.len() > 1 {
        println!("{} multiple .yk source files found in current directory:", "select:".green().bold());
        for (i, f) in found.iter().enumerate() {
            println!("  [{}] {}", i + 1, f.file_name().unwrap().to_str().unwrap());
        }
        print!("Choose file number to run [1-{}]: ", found.len());
        use std::io::Write;
        std::io::stdout().flush().ok();
        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_ok() {
            if let Ok(choice) = input.trim().parse::<usize>() {
                if choice >= 1 && choice <= found.len() {
                    return Ok(found[choice - 1].clone());
                }
            }
        }
        return Ok(found[0].clone());
    }

    bail!(
        "{} no entry file found — pass a path, or create `main.yk` / `src/main.yk`\n\
         hint: `york new hello` scaffolds a project",
        "error:".red().bold()
    );
}

/// Resolve the platform-specific executable path (append `.exe` on Windows).
pub fn resolve_exe(base: &Path) -> PathBuf {
    if cfg!(windows) {
        let s = base.display().to_string();
        PathBuf::from(format!("{s}.exe"))
    } else {
        base.to_path_buf()
    }
}

/// Fallback: invoke a user-specified C compiler directly.
pub fn compile_with_cc(source: &str, out: &Path, cc: &str) -> Result<()> {
    let c_file = std::env::temp_dir().join("york_build.c");
    std::fs::write(&c_file, source)?;
    let out_str = out.display().to_string();
    let status = process::Command::new(cc)
        .arg("-O2").arg("-std=c11").arg("-w")
        .arg(c_file.to_str().unwrap())
        .arg("-o").arg(&out_str)
        .status()
        .with_context(|| format!("failed to run C compiler `{cc}`"))?;
    let _ = std::fs::remove_file(&c_file);
    if status.success() {
        Ok(())
    } else {
        bail!("C compiler (`{cc}`) failed with status {status}");
    }
}