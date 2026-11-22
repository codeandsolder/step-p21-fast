//! Benchmark and compare parsing of real STEP files.
//!
//! Run with: `cargo run --release -p step-p21 --example parser_corpus --
//! [files]`. The verification mode hashes complete AST debug representations
//! outside timing measurements. Compare fingerprints using the same Rust
//! toolchain.

use std::{
    fmt::{self, Write},
    fs,
    hash::{DefaultHasher, Hasher},
    hint::black_box,
    path::{Path, PathBuf},
    time::Instant,
};

use anyhow::{Context, Result};
use clap::Parser;
use step_p21::{ast::Exchange, parser};

#[derive(Parser)]
struct Args {
    /// Hash the parsed AST instead of timing parse and drop.
    #[arg(long)]
    verify: bool,
    /// Number of measured parses per file.
    #[arg(long, default_value_t = 3, value_parser = clap::value_parser!(u32).range(1..))]
    iterations: u32,
    /// STEP files to parse (in the supplied order).
    #[arg(required = true)]
    files: Vec<PathBuf>,
}

struct AstFingerprint(DefaultHasher);

impl Write for AstFingerprint {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        self.0.write(text.as_bytes());
        Ok(())
    }
}

fn fingerprint(exchange: &Exchange) -> Result<u64> {
    let mut output = AstFingerprint(DefaultHasher::new());
    write!(output, "{exchange:?}")?;
    Ok(output.0.finish())
}

fn run(path: &Path, args: &Args) -> Result<()> {
    let data =
        fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let bytes = data.len();
    let input = match std::str::from_utf8(&data) {
        Ok(input) => input,
        Err(error) => {
            println!("invalid-utf8\t{bytes}\t{}", path.display());
            eprintln!("{}: {error}", path.display());
            return Ok(());
        }
    };
    let parsed = match parser::parse(input) {
        Ok(exchange) => exchange,
        Err(error) => {
            println!("rejected\t{bytes}\t{}", path.display());
            eprintln!(
                "{}: {}",
                path.display(),
                error.to_string().chars().take(240).collect::<String>()
            );
            return Ok(());
        }
    };
    if args.verify {
        let hash = fingerprint(&parsed)?;
        println!("verified\t{bytes}\t{hash:016x}\t{}", path.display());
    } else {
        // Drop the warm-up AST before measuring parse-and-drop repetitions.
        drop(parsed);
        for _ in 0..args.iterations {
            let start = Instant::now();
            drop(black_box(parser::parse(black_box(input))?));
            let elapsed = start.elapsed().as_nanos();
            println!("timed\t{bytes}\t{elapsed}\t{}", path.display());
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    for path in &args.files {
        run(path, &args)?;
    }
    Ok(())
}
