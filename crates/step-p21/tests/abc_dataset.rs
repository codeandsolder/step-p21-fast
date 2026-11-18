//! Integration tests for parsing STEP files from the ABC Dataset.

use nom::Finish;
use std::{fs, path::PathBuf};
use step_p21::parser;

fn format_example() -> anyhow::Result<String> {
    let step_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/steps/00000050_80d90bfdd2e74e709956122a_step_000.step");
    Ok(fs::read_to_string(step_file)?)
}

#[test]
fn abc_dataset_data_line() -> anyhow::Result<()> {
    for (index, line) in format_example()?
        .lines()
        .skip(7 /* lines before DATA; */)
        .take(1671 /* lines in DATA section */)
        .enumerate()
    {
        if let Err(error) = parser::exchange::entity_instance(line).finish() {
            anyhow::bail!("ABC STEP entity line {}: {error:?}", index + 8);
        }
    }
    Ok(())
}

#[test]
fn abc_dataset() -> anyhow::Result<()> {
    let step_str = format_example()?;
    let (residual, _exchange) = parser::exchange::exchange_file(&step_str)
        .finish()
        .map_err(|error| anyhow::anyhow!("ABC STEP file parse: {error:?}"))?;
    anyhow::ensure!(
        residual.trim().is_empty(),
        "ABC STEP parser left unconsumed input: {} bytes",
        residual.len()
    );
    Ok(())
}
