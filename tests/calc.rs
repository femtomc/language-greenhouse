use color_eyre::Report;
use language_greenhouse::calc::*;

#[test]
fn calc_0() -> Result<(), Report> {
    let expr = Expr::EMul(Box::new(Expr::Value(5)), Box::new(Expr::Value(5)));
    let v = Interpreter::new().eval(expr)?;
    assert!(v == 25);
    Ok(())
}

#[test]
fn calc_1() -> Result<(), Report> {
    let src = "let x = 5; x * x";
    let v = eval(src)?;
    assert!(v == 25);
    Ok(())
}

#[test]
fn calc_2() -> Result<(), Report> {
    let src = "let x = 5; (x + x) * x";
    let v = eval(src)?;
    assert!(v == 50);
    Ok(())
}
