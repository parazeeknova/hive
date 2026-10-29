//! Compile a local WAT capability fixture; never invoked by the production server.
fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let input = args
        .next()
        .ok_or_else(|| anyhow::anyhow!("usage: compile_wasm <input.wat> <output.wasm>"))?;
    let output = args
        .next()
        .ok_or_else(|| anyhow::anyhow!("output.wasm required"))?;
    if args.next().is_some() {
        anyhow::bail!("unexpected argument");
    }
    std::fs::write(output, wat::parse_file(input)?)?;
    Ok(())
}
