use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use compiler::abi_codegen::AbiCodeGenerator;
use compiler::abi_generator::AbiGenerator;

#[derive(Debug, Clone)]
struct Paths {
    repo_root: PathBuf,
    target_json: PathBuf,
    linker_script: PathBuf,
    manifest_path: PathBuf,
}

impl Paths {
    fn discover() -> Self {
        // CARGO_MANIFEST_DIR points at crates/compiler
        let compiler_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let repo_root = compiler_dir
            .parent()
            .and_then(|p| p.parent())
            .expect("Failed to find repo root")
            .to_path_buf();

        let target_json = repo_root.join("crates/compiler/targets/avm32.json");
        let linker_script = repo_root.join("crates/compiler/linker.ld");

        // Prefer a manifest in the current working directory; fall back to the workspace root.
        let cwd = env::current_dir().unwrap_or_else(|_| repo_root.clone());
        let mut manifest_path = cwd.join("Cargo.toml");
        if !manifest_path.exists() {
            let fallback_workspace = repo_root.join("Cargo.toml");
            manifest_path = fallback_workspace;
        }

        Self {
            repo_root,
            target_json,
            linker_script,
            manifest_path,
        }
    }
}

fn main() {
    let mut args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        print_usage();
        std::process::exit(1);
    }

    let paths = Paths::discover();
    let cmd = args.remove(0);
    let result = match cmd.as_str() {
        "build" => cmd_build(args, &paths),
        "abi" => cmd_abi(args, &paths),
        "client" => cmd_client(args, &paths),
        "all" => cmd_all(args, &paths),
        _ => {
            print_usage();
            Err("unknown command".to_string())
        }
    };

    if let Err(e) = result {
        eprintln!("✗ {}", e);
        std::process::exit(1);
    }
}

fn cmd_build(args: Vec<String>, paths: &Paths) -> Result<(), String> {
    let mut bin: Option<String> = None;
    let mut features: Option<String> = None;
    let mut release = true;
    let mut out_dir: Option<PathBuf> = None;
    let mut cargo_cmd: Option<String> = None;
    let mut manifest_path: Option<PathBuf> = None;
    let mut linker_script: Option<PathBuf> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--bin" => {
                i += 1;
                bin = Some(args.get(i).cloned().ok_or("missing value for --bin")?);
            }
            "--features" => {
                i += 1;
                features = Some(
                    args.get(i)
                        .cloned()
                        .ok_or("missing value for --features")?,
                );
            }
            "--debug" => release = false,
            "--release" => release = true,
            "--out-dir" => {
                i += 1;
                let val = args.get(i).cloned().ok_or("missing value for --out-dir")?;
                out_dir = Some(PathBuf::from(val));
            }
            "--cargo" => {
                i += 1;
                cargo_cmd =
                    Some(args.get(i).cloned().ok_or("missing value for --cargo")?);
            }
            "--manifest-path" => {
                i += 1;
                let val = args.get(i).cloned().ok_or("missing value for --manifest-path")?;
                manifest_path = Some(PathBuf::from(val));
            }
            "--linker-script" => {
                i += 1;
                let val = args.get(i).cloned().ok_or("missing value for --linker-script")?;
                linker_script = Some(PathBuf::from(val));
            }
            other => return Err(format!("unknown flag {}", other)),
        }
        i += 1;
    }

    let bin = bin.ok_or("missing --bin <name>")?;
    let manifest_path = manifest_path.unwrap_or_else(|| paths.manifest_path.clone());
    let manifest_dir = manifest_path
        .parent()
        .map(|p| p.to_path_buf())
        .ok_or("invalid manifest path")?;
    if !manifest_path.exists() {
        return Err(format!(
            "manifest path {} does not exist",
            manifest_path.display()
        ));
    }
    let out_dir = out_dir.unwrap_or_else(|| manifest_dir.join("bin"));
    fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;

    let cargo = cargo_cmd
        .or_else(|| env::var("CARGO_NIGHTLY").ok())
        .unwrap_or_else(|| "cargo".to_string());

    let mut rustflags = env::var("RUSTFLAGS").unwrap_or_default();
    if !rustflags.is_empty() {
        rustflags.push(' ');
    }
    rustflags.push_str("-Clink-arg=-T");
    let linker = linker_script.unwrap_or_else(|| paths.linker_script.clone());
    if !linker.exists() {
        return Err(format!(
            "linker script {} does not exist",
            linker.display()
        ));
    }
    rustflags.push_str(
        &linker
            .to_str()
            .ok_or("invalid linker script path")?,
    );

    // Split cargo command to support "+nightly" prefixes
    let mut parts = cargo.split_whitespace();
    let cmd = parts
        .next()
        .ok_or("invalid cargo command")?
        .to_string();
    let mut cargo_args: Vec<String> = parts.map(|s| s.to_string()).collect();
    cargo_args.push("build".to_string());
    cargo_args.push("--manifest-path".to_string());
    cargo_args.push(manifest_path.display().to_string());
    cargo_args.push("--target".to_string());
    cargo_args.push(paths.target_json.display().to_string());
    if release {
        cargo_args.push("--release".to_string());
    }
    cargo_args.push("-Zbuild-std=core,alloc,compiler_builtins".to_string());
    cargo_args.push("-Zbuild-std-features=compiler-builtins-mem".to_string());
    cargo_args.push("--bin".to_string());
    cargo_args.push(bin.clone());
    cargo_args.push("--features".to_string());
    cargo_args.push(features.unwrap_or_else(|| "binaries".to_string()));

    let status = Command::new(cmd)
        .args(&cargo_args)
        .env("RUSTFLAGS", rustflags)
        .status()
        .map_err(|e| format!("failed to run cargo: {}", e))?;

    if !status.success() {
        return Err(format!("cargo build failed with status {:?}", status.code()));
    }

    let target_dir = paths
        .repo_root
        .join("target")
        .join("avm32")
        .join(if release { "release" } else { "debug" });
    let built = target_dir.join(&bin);
    if !built.exists() {
        return Err(format!(
            "expected built artifact at {}",
            built.display()
        ));
    }

    let dest = out_dir.join(format!("{}.elf", bin));
    fs::copy(&built, &dest)
        .map_err(|e| format!("failed to copy {} to {}: {}", built.display(), dest.display(), e))?;

    println!("✓ Built {} -> {}", bin, dest.display());
    Ok(())
}

fn cmd_abi(args: Vec<String>, paths: &Paths) -> Result<(), String> {
    let mut bin: Option<String> = None;
    let mut src: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut manifest_path: Option<PathBuf> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--bin" => {
                i += 1;
                bin = Some(args.get(i).cloned().ok_or("missing value for --bin")?);
            }
            "--src" => {
                i += 1;
                let val = args.get(i).cloned().ok_or("missing value for --src")?;
                src = Some(PathBuf::from(val));
            }
            "--out" => {
                i += 1;
                let val = args.get(i).cloned().ok_or("missing value for --out")?;
                out = Some(PathBuf::from(val));
            }
            "--manifest-path" => {
                i += 1;
                let val = args.get(i).cloned().ok_or("missing value for --manifest-path")?;
                manifest_path = Some(PathBuf::from(val));
            }
            other => return Err(format!("unknown flag {}", other)),
        }
        i += 1;
    }

    let manifest_path = manifest_path.unwrap_or_else(|| paths.manifest_path.clone());
    let manifest_dir = manifest_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| paths.repo_root.clone());

    let (bin_name, src_path) = if let Some(s) = src {
        let name = bin
            .or_else(|| {
                s.file_stem()
                    .and_then(|f| f.to_str())
                    .map(|s| s.to_string())
            })
            .ok_or("could not infer --bin name from --src")?;
        (name, s)
    } else {
        let name = bin.ok_or("missing --bin <name> or --src <path>")?;
        let inferred = manifest_dir.join("src").join(format!("{}.rs", name));
        (name, inferred)
    };

    let output = out.unwrap_or_else(|| manifest_dir.join("bin").join(format!("{}.abi.json", bin_name)));
    let source =
        fs::read_to_string(&src_path).map_err(|e| format!("failed to read {}: {}", src_path.display(), e))?;

    let mut generator = AbiGenerator::new(source);
    let abi = generator.generate();
    fs::create_dir_all(
        output
            .parent()
            .ok_or("invalid output path for abi")?,
    )
    .map_err(|e| e.to_string())?;
    fs::write(&output, abi.to_json())
        .map_err(|e| format!("failed to write {}: {}", output.display(), e))?;

    println!("✓ Generated ABI {}", output.display());
    Ok(())
}

fn cmd_client(args: Vec<String>, _paths: &Paths) -> Result<(), String> {
    let mut abi_path: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut contract: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--abi" => {
                i += 1;
                let val = args.get(i).cloned().ok_or("missing value for --abi")?;
                abi_path = Some(PathBuf::from(val));
            }
            "--out" => {
                i += 1;
                let val = args.get(i).cloned().ok_or("missing value for --out")?;
                out = Some(PathBuf::from(val));
            }
            "--contract" => {
                i += 1;
                contract = Some(args.get(i).cloned().ok_or("missing value for --contract")?);
            }
            other => return Err(format!("unknown flag {}", other)),
        }
        i += 1;
    }

    let abi_path = abi_path.ok_or("missing --abi <path>")?;
    let out = out.unwrap_or_else(|| {
        let stem = abi_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("client");
        abi_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(format!("{}_abi.rs", stem.trim_end_matches(".abi")))
    });

    let contract_name = contract.unwrap_or_else(|| derive_contract_name(&abi_path));

    let code = AbiCodeGenerator::from_abi_file(
        abi_path
            .to_str()
            .ok_or("invalid abi path")?,
        contract_name,
    )
    .map_err(|e| format!("failed to generate client: {}", e))?;

    fs::create_dir_all(out.parent().ok_or("invalid output path for client")?)
        .map_err(|e| e.to_string())?;
    fs::write(&out, code)
        .map_err(|e| format!("failed to write {}: {}", out.display(), e))?;

    println!("✓ Generated client {}", out.display());
    Ok(())
}

fn cmd_all(args: Vec<String>, paths: &Paths) -> Result<(), String> {
    let mut bin: Option<String> = None;
    let mut out_dir: Option<PathBuf> = None;
    let mut cargo_cmd: Option<String> = None;
    let mut features: Option<String> = None;
    let mut manifest_path: Option<PathBuf> = None;
    let mut src_override: Option<PathBuf> = None;
    let mut linker_script: Option<PathBuf> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--bin" => {
                i += 1;
                bin = Some(args.get(i).cloned().ok_or("missing value for --bin")?);
            }
            "--out-dir" => {
                i += 1;
                let val = args.get(i).cloned().ok_or("missing value for --out-dir")?;
                out_dir = Some(PathBuf::from(val));
            }
            "--cargo" => {
                i += 1;
                cargo_cmd =
                    Some(args.get(i).cloned().ok_or("missing value for --cargo")?);
            }
            "--features" => {
                i += 1;
                features = Some(
                    args.get(i)
                        .cloned()
                        .ok_or("missing value for --features")?,
                );
            }
            "--manifest-path" => {
                i += 1;
                let val = args.get(i).cloned().ok_or("missing value for --manifest-path")?;
                manifest_path = Some(PathBuf::from(val));
            }
            "--src" => {
                i += 1;
                let val = args.get(i).cloned().ok_or("missing value for --src")?;
                src_override = Some(PathBuf::from(val));
            }
            "--linker-script" => {
                i += 1;
                let val = args.get(i).cloned().ok_or("missing value for --linker-script")?;
                linker_script = Some(PathBuf::from(val));
            }
            other => return Err(format!("unknown flag {}", other)),
        }
        i += 1;
    }

    let bin = bin.ok_or("missing --bin <name>")?;
    let manifest_path = manifest_path.unwrap_or_else(|| paths.manifest_path.clone());
    let manifest_dir = manifest_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| paths.repo_root.clone());
    let out_dir = out_dir.unwrap_or_else(|| manifest_dir.join("bin"));
    let abi_out = out_dir.join(format!("{}.abi.json", bin));
    let client_out = out_dir.join(format!("{}_abi.rs", bin));
    let src = src_override.unwrap_or_else(|| manifest_dir.join("src").join(format!("{}.rs", bin)));

    cmd_build(
        vec![
            "--bin".into(),
            bin.clone(),
            "--out-dir".into(),
            out_dir.display().to_string(),
            "--features".into(),
            features.unwrap_or_else(|| "binaries".to_string()),
            "--release".into(),
            "--cargo".into(),
            cargo_cmd.unwrap_or_else(|| {
                env::var("CARGO_NIGHTLY").unwrap_or_else(|_| "cargo".to_string())
            }),
            "--manifest-path".into(),
            manifest_path.display().to_string(),
            "--linker-script".into(),
            linker_script
                .unwrap_or_else(|| paths.linker_script.clone())
                .display()
                .to_string(),
        ],
        paths,
    )?;

    cmd_abi(
        vec![
            "--bin".into(),
            bin.clone(),
            "--src".into(),
            src.display().to_string(),
            "--out".into(),
            abi_out.display().to_string(),
            "--manifest-path".into(),
            manifest_path.display().to_string(),
        ],
        paths,
    )?;

    cmd_client(
        vec![
            "--abi".into(),
            abi_out.display().to_string(),
            "--out".into(),
            client_out.display().to_string(),
            "--contract".into(),
            derive_contract_name_from_bin(&bin),
        ],
        paths,
    )?;

    Ok(())
}

fn derive_contract_name(abi_path: &Path) -> String {
    let stem = abi_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("contract");
    let trimmed = stem.trim_end_matches(".abi");
    format!("{}Contract", capitalize_first(trimmed))
}

fn derive_contract_name_from_bin(bin: &str) -> String {
    format!("{}Contract", capitalize_first(bin))
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().chain(chars).collect(),
    }
}

fn print_usage() {
    eprintln!(
        "Usage:
  avm32 build --bin <name> [--manifest-path <cargo_toml>] [--out-dir <dir>] [--linker-script <file>] [--cargo <cmd>] [--features <feat>] [--debug|--release]
  avm32 abi --bin <name> [--src <path>] [--out <file>] [--manifest-path <cargo_toml>]
  avm32 client --abi <file> [--out <file>] [--contract <name>]
  avm32 all --bin <name> [--manifest-path <cargo_toml>] [--src <path>] [--out-dir <dir>] [--linker-script <file>] [--cargo <cmd>] [--features <feat>]"
    );
}
