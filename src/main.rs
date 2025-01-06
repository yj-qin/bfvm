mod crane_jit;
mod fast_jit;
mod interpreter;
mod parser;

use clap::Parser;
use interpreter::Interpreter;
use std::fs::File;
use std::io::{Read, Write};
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
    // Crane JIT
    #[arg(long)]
    crane_jit: bool,
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
    } else if args.crane_jit {
        let mut program = match crane_jit::Program::new(&source) {
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

pub(crate) extern "sysv64" fn write(value: u8) -> *mut std::io::Error {
    // Writing a non-UTF-8 byte sequence on Windows error out.
    if cfg!(target_os = "windows") && value >= 128 {
        return std::ptr::null_mut();
    }

    let mut stdout = std::io::stdout().lock();

    let result = stdout.write_all(&[value]).and_then(|_| stdout.flush());

    match result {
        Err(err) => Box::into_raw(Box::new(err)),
        _ => std::ptr::null_mut(),
    }
}

pub(crate) unsafe extern "sysv64" fn read(buf: *mut u8) -> *mut std::io::Error {
    let mut stdin = std::io::stdin().lock();
    loop {
        let mut value = 0;
        let err = stdin.read_exact(std::slice::from_mut(&mut value));

        if let Err(err) = err {
            if err.kind() != std::io::ErrorKind::UnexpectedEof {
                return Box::into_raw(Box::new(err));
            }
            value = 0;
        }

        // ignore CR from Window's CRLF
        if cfg!(target_os = "windows") && value == b'\r' {
            continue;
        }

        *buf = value;

        return std::ptr::null_mut();
    }
}
