use color_eyre::{eyre::bail, eyre::eyre, Report};
use language_greenhouse::func;
use language_greenhouse::func::*;

#[test]
fn func_1() -> Result<(), Report> {
    let src = "let fn = y . 5 * (y + 5); fn(5)";
    let v = eval(src)?;
    match v {
        Value::VInt(value) => assert!(value == 50),
        _ => bail!("Error."),
    }
    Ok(())
}
