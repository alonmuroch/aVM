use compiler::abi_generator::AbiGenerator;
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <input.rs> <output.abi.json>", args[0]);
        std::process::exit(1);
    }

    let input_file = &args[1];
    let output_file = &args[2];

    println!("Generating ABI for {input_file}");

    match fs::read_to_string(input_file) {
        Ok(source_code) => {
            let mut generator = AbiGenerator::new(source_code);
            let abi = generator.generate();

            match fs::write(output_file, abi.to_json()) {
                Ok(_) => println!("✓ Generated ABI: {output_file}"),
                Err(e) => {
                    eprintln!("✗ Failed to write ABI: {e}");
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("✗ Failed to read input file: {e}");
            std::process::exit(1);
        }
    }
}
