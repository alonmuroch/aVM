use compiler::abi_codegen::AbiCodeGenerator;
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!(
            "Usage: {} <abi-file> <output-file> [contract-name]",
            args[0]
        );
        eprintln!(
            "Example: {} simple.abi.json simple_client.rs SimpleContract",
            args[0]
        );
        std::process::exit(1);
    }

    let abi_file = &args[1];
    let output_file = &args[2];
    let contract_name = if args.len() > 3 {
        args[3].clone()
    } else {
        // Derive contract name from filename
        Path::new(abi_file)
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.replace(".abi", ""))
            .map(|s| format!("{}Contract", capitalize_first(&s)))
            .unwrap_or_else(|| "Contract".to_string())
    };

    println!(
        "Generating client code for {} as {}",
        abi_file, contract_name
    );

    match AbiCodeGenerator::from_abi_file(abi_file, contract_name) {
        Ok(code) => {
            if let Err(e) = fs::write(output_file, code) {
                eprintln!("Failed to write output file: {}", e);
                std::process::exit(1);
            }
            println!("✓ Generated client code: {}", output_file);
        }
        Err(e) => {
            eprintln!("Failed to generate client code: {}", e);
            std::process::exit(1);
        }
    }
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().chain(chars).collect(),
    }
}
