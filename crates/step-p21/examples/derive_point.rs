//! Decode a STEP record directly into a derived Rust entity.

#[derive(Debug, PartialEq, step_p21::Deserialize)]
pub struct Point {
    x: i64,
}

fn main() -> anyhow::Result<()> {
    let (_, record) = step_p21::parser::exchange::simple_record("POINT(42)")
        .map_err(|error| anyhow::anyhow!("{error:?}"))?;
    let point: Point = serde::Deserialize::deserialize(&record)?;
    assert_eq!(point, Point { x: 42 });
    Ok(())
}
