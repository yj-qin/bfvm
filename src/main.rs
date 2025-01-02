mod fast_jit;
mod interpreter;
mod parser;

use clap::Parser;
use interpreter::Interpreter;
use std::fs::File;
use std::io::Read;
use std::process::exit;

const INIT_MEMORY_SIZE: usize = 4096000;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    // Brainfuck source path
    #[arg(required = true)]
    path: String,
    // Debug mode
    #[arg(short, long)]
    debug: bool,
    // Fast JIT
    #[arg(long)]
    fast_jit: bool,
}

fn main() {
    let args = Args::parse();

    let source = read_file(&args.path).unwrap_or_else(|e| {
        eprintln!("Load program error: {}", e);
        exit(1)
    });

    if args.fast_jit {
        let program = match fast_jit::Program::new(&source) {
            Ok(p) => p,
            Err(err) => {
                eprintln!("Compile error: {}", err);
                exit(1)
            }
        };
        program.run().unwrap_or_else(|err| {
            eprintln!("Runtime error: {}", err);
            exit(1)
        });
    } else {
        let mut interpreter = Interpreter::new();
        interpreter.run(&source).unwrap_or_else(|err| {
            eprintln!("Runtime error: {}", err);
        });
    }
}

fn read_file(path: &str) -> Result<String, String> {
    let mut buffer = String::new();
    let mut file = File::open(path).map_err(|e| format!("Could not open file: {:?}", e))?;
    file.read_to_string(&mut buffer)
        .map_err(|e| format!("Could not read file: {:?}", e))?;
    Ok(buffer)
}
