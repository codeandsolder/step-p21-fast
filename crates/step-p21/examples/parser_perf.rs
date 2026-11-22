//! Reproducible release-mode parser microbenchmarks.
//! Run with cargo run --release -p step-p21 --example parser_perf.

use std::{hint::black_box, time::Instant};

use anyhow::{Result, anyhow};
use step_p21::parser::{self, token};

fn measure(
    name: &str,
    count: u32,
    mut run: impl FnMut() -> Result<()>,
) -> Result<()> {
    for _ in 0..3 {
        run()?;
    }
    for _ in 0..7 {
        let start = Instant::now();
        for _ in 0..count {
            run()?;
        }
        println!(
            "{name} {:.1} ns",
            start.elapsed().as_secs_f64() * 1e9 / f64::from(count)
        );
    }
    Ok(())
}

fn main() -> Result<()> {
    let fixture = include_str!(
        "../tests/steps/00000050_80d90bfdd2e74e709956122a_step_000.step"
    );
    let keyword = "ADVANCED_FACE";
    let long_keyword =
        "GEOMETRIC_REPRESENTATION_CONTEXT_WITH_PARAMETER".repeat(8);
    let string = format!(
        "'{}'",
        "A long descriptive STEP string containing unicode é and escaped '' apostrophes. ".repeat(20)
    );

    measure("fixture", 100, || {
        black_box(parser::parse(black_box(fixture))?);
        Ok(())
    })?;
    for (name, input, count) in [
        ("short_keyword", keyword, 300_000),
        ("long_keyword", long_keyword.as_str(), 100_000),
    ] {
        measure(name, count, || {
            black_box(
                token::standard_keyword(black_box(input))
                    .map_err(|error| anyhow!("{error:?}"))?,
            );
            Ok(())
        })?;
    }
    measure("string", 50_000, || {
        black_box(
            token::string(black_box(&string))
                .map_err(|error| anyhow!("{error:?}"))?,
        );
        Ok(())
    })
}
