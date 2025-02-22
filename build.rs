use anyhow::{Ok, Result};
use substreams_ethereum::Abigen;

fn main() -> Result<(), anyhow::Error> {
    Abigen::new("tornado_cash", "abi/abi.json")?
        .generate()?
        .write_to_file("src/abi/tornado_cash.rs")?;

    Ok(())
}
