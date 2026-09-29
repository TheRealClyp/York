use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use colored::Colorize;
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::pipeline;

/// Watch a project for `.yk` changes and rebuild+run on each change.
pub fn dev(root: &Path, entry: &Path, args: &[String]) -> Result<()> {
    // First build & run so the user sees output immediately.
    run_once(root, entry, args);

    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = RecommendedWatcher::new(
        tx,
        Config::default().with_poll_interval(Duration::from_millis(400)),
    )?;
    watcher.watch(root, RecursiveMode::Recursive)?;

    let running = Arc::new(AtomicBool::new(false));
    let stop = Arc::new(AtomicBool::new(false));

    loop {
        let event = match rx.recv_timeout(Duration::from_millis(300)) {
            Ok(Ok(event)) => event,
            Ok(Err(e)) => {
                eprintln!("{} {e}", "watcher error:".red().bold());
                continue;
            }
            Err(_) => {
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                continue;
            }
        };

        // Only react to modified .yk files (ignore temp dirs, target/, .git).
        let relevant = match event.kind {
            EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_) => true,
            _ => false,
        };
        if !relevant {
            continue;
        }
        let is_yk = event
            .paths
            .iter()
            .any(|p| p.extension().and_then(|e| e.to_str()) == Some("yk"));
        if !is_yk {
            continue;
        }
        if event.paths.iter().any(|p| {
            let s = p.to_string_lossy();
            s.contains("target") || s.contains(".git") || s.contains("node_modules")
        }) {
            continue;
        }

        // Debounce: wait a beat, then rebuild once.
        std::thread::sleep(Duration::from_millis(120));
        run_once(root, entry, args);
        let _ = running;
    }
    Ok(())
}

fn run_once(root: &Path, entry: &Path, args: &[String]) {
    let t = std::time::Instant::now();
    println!();
    println!("{} rebuilding…", "▶".cyan().bold());
    match pipeline::compile_run(entry, args) {
        Ok(elapsed) => {
            println!("{} compiled in {:.2?}", "ok".green().bold(), elapsed);
            if entry == root {
                println!("  watching `{}` for changes", root.display());
            } else {
                println!("  watching `{}` for changes", root.display());
            }
        }
        Err(e) => {
            eprintln!("{} {e}", "error:".red().bold());
        }
    }
}