use aaiyyan_ast::{BinaryOp, Expr, Statement};
use aaiyyan_parser::parse_program;

#[test]
fn trailing_equals_is_evaluation() {
    let stmts = parse_program("2+3=").unwrap();
    assert_eq!(stmts.len(), 1);
    match &stmts[0] {
        Statement::Expr(Expr::Binary { op, .. }) => assert_eq!(*op, BinaryOp::Add),
        other => panic!("expected an evaluation statement, got {other:?}"),
    }
}

#[test]
fn trailing_equals_tolerates_whitespace_and_newlines() {
    assert!(parse_program("2+3 = ").is_ok());
    assert!(parse_program("2+3 =\n").is_ok());
    let stmts = parse_program("x =\ny := 2").unwrap();
    assert_eq!(stmts.len(), 2);
    assert!(matches!(stmts[0], Statement::Expr(_)));
    assert!(matches!(stmts[1], Statement::Assign { .. }));
}

#[test]
fn equals_with_right_hand_side_is_still_comparison() {
    let stmts = parse_program("a = 5").unwrap();
    match &stmts[0] {
        Statement::Expr(Expr::Binary { op, .. }) => assert_eq!(*op, BinaryOp::Eq),
        other => panic!("expected a comparison, got {other:?}"),
    }
    let neg = parse_program("a = -1").unwrap();
    assert!(matches!(neg[0], Statement::Expr(Expr::Binary { .. })));
}

#[test]
fn definitions_and_units_are_unaffected() {
    let stmts = parse_program("E := 210 GPa\ndelta := 5*w*L^4/(384*E*I)\ndelta =").unwrap();
    assert!(matches!(stmts[0], Statement::Assign { .. }));
    assert!(matches!(stmts[1], Statement::Assign { .. }));
    assert!(matches!(stmts[2], Statement::Expr(_)));
}
