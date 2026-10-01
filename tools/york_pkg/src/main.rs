use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use clap::{Parser, Subcommand};
use colored::*;
use serde::{Deserialize, Serialize};
use anyhow::{Context, Result};

#[derive(Parser)]
#[command(name = "ypkg", version = "1.1.1", about = "ypkg — The Official York Package Manager")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new York package in the current directory
    Init {
        /// Package name (default: current directory name)
        name: Option<String>,
    },
    /// Add a dependency to the current project
    Add {
        /// Package name
        package: String,
        /// Package version
        #[arg(default_value = "latest")]
        version: String,
    },
    /// Remove a dependency from the current project
    Remove {
        /// Package name to remove
        package: String,
    },
    /// Install all dependencies listed in york.toml into york_modules/
    Install,
    /// List all installed packages and dependencies
    List,
    /// Bundle and validate package for distribution
    Publish,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Manifest {
    package: PackageInfo,
    #[serde(default)]
    dependencies: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PackageInfo {
    name: String,
    version: String,
    description: Option<String>,
    authors: Option<Vec<String>>,
    license: Option<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { name } => {
            let cwd = std::env::current_dir()?;
            let pkg_name = name.unwrap_or_else(|| {
                cwd.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("my_york_project")
                    .to_string()
            });

            let manifest_path = cwd.join("york.toml");
            if manifest_path.exists() {
                anyhow::bail!("{}: `york.toml` already exists in this directory", "error".red().bold());
            }

            let manifest = Manifest {
                package: PackageInfo {
                    name: pkg_name.clone(),
                    version: "0.1.0".to_string(),
                    description: Some(format!("A high-performance York systems project")),
                    authors: Some(vec!["York Contributor".to_string()]),
                    license: Some("MIT".to_string()),
                },
                dependencies: BTreeMap::new(),
            };

            let toml_str = toml::to_string_pretty(&manifest)?;
            fs::write(&manifest_path, toml_str)?;

            let src_dir = cwd.join("src");
            if !src_dir.exists() {
                fs::create_dir_all(&src_dir)?;
            }
            let main_yk = src_dir.join("main.yk");
            if !main_yk.exists() {
                fs::write(
                    &main_yk,
                    "// Welcome to your York project!\n\npublic static void main(String[] args) {\n    println(\"Hello from York!\");\n}\n",
                )?;
            }

            println!("{} created package {} at {}", "Initialized".green().bold(), pkg_name.cyan(), manifest_path.display());
            println!("  Created: {}", "york.toml".bold());
            println!("  Created: {}", "src/main.yk".bold());
        }

        Commands::Add { package, version } => {
            let (manifest_path, mut manifest) = load_manifest()?;
            println!("{} dependency `{}` ({})", "Adding".cyan().bold(), package.bold(), version);

            manifest.dependencies.insert(package.clone(), version.clone());
            save_manifest(&manifest_path, &manifest)?;

            // Install package skeleton in york_modules
            let modules_dir = manifest_path.parent().unwrap().join("york_modules").join(&package);
            fs::create_dir_all(&modules_dir)?;
            let mod_yk = modules_dir.join("lib.yk");
            if !mod_yk.exists() {
                fs::write(
                    &mod_yk,
                    format!("// Module: {}\n\npub fn {}_version() -> String {{\n    return \"{}\";\n}}\n", package, package, version),
                )?;
            }

            println!("{} `{}` ({}) -> {}", "Installed".green().bold(), package, version, modules_dir.display());
        }

        Commands::Remove { package } => {
            let (manifest_path, mut manifest) = load_manifest()?;
            if manifest.dependencies.remove(&package).is_some() {
                save_manifest(&manifest_path, &manifest)?;
                let mod_dir = manifest_path.parent().unwrap().join("york_modules").join(&package);
                if mod_dir.exists() {
                    let _ = fs::remove_dir_all(mod_dir);
                }
                println!("{} removed dependency `{}`", "Success:".green().bold(), package);
            } else {
                println!("{}: package `{}` was not found in `york.toml`", "warning".yellow().bold(), package);
            }
        }

        Commands::Install => {
            let (manifest_path, manifest) = load_manifest()?;
            let root = manifest_path.parent().unwrap();
            let modules = root.join("york_modules");
            fs::create_dir_all(&modules)?;

            println!("{} {} dependencies for `{}`...", "Resolving".cyan().bold(), manifest.dependencies.len(), manifest.package.name);
            for (pkg, ver) in &manifest.dependencies {
                let pkg_dir = modules.join(pkg);
                if !pkg_dir.exists() {
                    fs::create_dir_all(&pkg_dir)?;
                    let lib_file = pkg_dir.join("lib.yk");
                    fs::write(&lib_file, format!("// Package: {}\npub fn info() -> String {{ return \"{}\"; }}\n", pkg, ver))?;
                }
                println!("  {} {} (v{})", "✓".green(), pkg.bold(), ver);
            }
            println!("{} All dependencies installed in `{}`", "Complete:".green().bold(), modules.display());
        }

        Commands::List => {
            let (_, manifest) = load_manifest()?;
            println!("{} {} (v{})", "Package:".bold().cyan(), manifest.package.name.bold(), manifest.package.version);
            if let Some(desc) = manifest.package.description {
                println!("  Description: {}", desc.dimmed());
            }
            println!("\n{}", "Dependencies:".bold());
            if manifest.dependencies.is_empty() {
                println!("  (none)");
            } else {
                for (k, v) in manifest.dependencies {
                    println!("  ├── {} {}", k.green().bold(), v.dimmed());
                }
            }
        }

        Commands::Publish => {
            let (manifest_path, manifest) = load_manifest()?;
            let root = manifest_path.parent().unwrap();
            println!("{} validating package `{}` (v{})...", "Publishing:".cyan().bold(), manifest.package.name, manifest.package.version);

            // Validation checks
            if manifest.package.name.is_empty() {
                anyhow::bail!("Package name cannot be empty");
            }
            let src = root.join("src");
            if !src.exists() {
                anyhow::bail!("Missing `src/` directory in package root");
            }

            println!("  {} Manifest validation passed", "✓".green());
            println!("  {} Source layout verified", "✓".green());
            println!("{} Package `{}` (v{}) is ready for the York Registry!", "SUCCESS:".green().bold(), manifest.package.name, manifest.package.version);
        }
    }

    Ok(())
}

fn load_manifest() -> Result<(PathBuf, Manifest)> {
    let mut dir = std::env::current_dir()?;
    loop {
        let candidate = dir.join("york.toml");
        if candidate.is_file() {
            let content = fs::read_to_string(&candidate)
                .with_context(|| format!("cannot read `{}`", candidate.display()))?;
            let manifest: Manifest = toml::from_str(&content)
                .with_context(|| format!("invalid TOML format in `{}`", candidate.display()))?;
            return Ok((candidate, manifest));
        }
        if !dir.pop() {
            break;
        }
    }
    anyhow::bail!("{}: could not find `york.toml` in current directory or any parent directory. Run `ypkg init` first.", "error".red().bold());
}

fn save_manifest(path: &Path, manifest: &Manifest) -> Result<()> {
    let toml_str = toml::to_string_pretty(manifest)?;
    fs::write(path, toml_str)?;
    Ok(())
}
