use awa5_rs::*;

#[test]
fn test_parse_program_basic() {
    let src = "module M = struct end;; include Foo;; extern foo;; let x = 1;; fn f ;;";
    let prog = parse_program(src).expect("parse_program failed");
    assert_eq!(prog.items.len(), 5);

    match &prog.items[0] {
        AstItem::ModuleDecl(m) => assert_eq!(m.name, "M"),
        other => panic!("expected ModuleDecl, got {:?}", other),
    }

    match &prog.items[1] {
        AstItem::IncludeStmt(_) => (),
        other => panic!("expected IncludeStmt, got {:?}", other),
    }

    match &prog.items[2] {
        AstItem::ExternDecl(e) => assert_eq!(e.name, "foo"),
        other => panic!("expected ExternDecl, got {:?}", other),
    }

    match &prog.items[3] {
        AstItem::LetDecl(l) => assert_eq!(l.name, "x"),
        other => panic!("expected LetDecl, got {:?}", other),
    }

    match &prog.items[4] {
        AstItem::FnDecl(f) => assert_eq!(f.name, "f"),
        other => panic!("expected FnDecl, got {:?}", other),
    }
}

#[test]
fn test_parse_fun_expr_in_let() {
    let item = parse_item("let id = fun x -> x").expect("parse_item failed");
    match item {
        AstItem::LetDecl(ld) => match ld.value {
            Expr::Fun { params, body } => {
                assert_eq!(params.len(), 1);
                assert!(matches!(params[0], Pattern::Ident(ref n) if n == "x"));
                assert!(matches!(*body, Expr::Ident(ref n) if n == "x"));
            }
            other => panic!("expected Fun expr, got {:?}", other),
        },
        other => panic!("expected LetDecl, got {:?}", other),
    }
}

#[test]
fn test_parse_match_expr_in_let() {
    let item = parse_item("let n = match x with | _ -> 1 | y when y -> 2").expect("parse_item failed");
    match item {
        AstItem::LetDecl(ld) => match ld.value {
            Expr::Match { value, cases } => {
                assert!(matches!(*value, Expr::Ident(ref n) if n == "x"));
                assert_eq!(cases.len(), 2);
                assert!(matches!(cases[0].pattern, Pattern::Wildcard));
                assert!(matches!(cases[0].body, Expr::Int(1)));
                assert!(matches!(cases[1].pattern, Pattern::Ident(ref n) if n == "y"));
                assert!(matches!(cases[1].guard, Some(Expr::Ident(ref n)) if n == "y"));
                assert!(matches!(cases[1].body, Expr::Int(2)));
            }
            other => panic!("expected Match expr, got {:?}", other),
        },
        other => panic!("expected LetDecl, got {:?}", other),
    }
}

#[test]
fn test_precedence_in_let_expr() {
    let item = parse_item("let x = 1 + 2 * 3 - 4 / 2").expect("parse_item failed");
    match item {
        AstItem::LetDecl(ld) => match ld.value {
            Expr::Call { callee, args } => {
                assert_eq!(callee, "-");
                assert_eq!(args.len(), 2);
                assert!(matches!(args[1], Expr::Call { ref callee, .. } if callee == "/"));
                match &args[0] {
                    Expr::Call { callee, args } => {
                        assert_eq!(callee, "+");
                        assert!(matches!(args[1], Expr::Call { ref callee, .. } if callee == "*"));
                    }
                    other => panic!("expected addition on left side, got {:?}", other),
                }
            }
            other => panic!("expected binary-call expr, got {:?}", other),
        },
        other => panic!("expected LetDecl, got {:?}", other),
    }
}

#[test]
fn test_alias_constructor_pattern() {
    let item = parse_item("let r = match v with | Some(x) as y -> y | _ -> v").expect("parse_item failed");
    match item {
        AstItem::LetDecl(ld) => match ld.value {
            Expr::Match { cases, .. } => {
                assert_eq!(cases.len(), 2);
                match &cases[0].pattern {
                    Pattern::As { pattern, alias } => {
                        assert_eq!(alias, "y");
                        match &**pattern {
                            Pattern::Constructor { name, payload } => {
                                assert_eq!(name, "Some");
                                assert!(matches!(payload.as_deref(), Some(Pattern::Ident(ref n)) if n == "x"));
                            }
                            other => panic!("expected constructor pattern, got {:?}", other),
                        }
                    }
                    other => panic!("expected alias pattern, got {:?}", other),
                }
            }
            other => panic!("expected Match expr, got {:?}", other),
        },
        other => panic!("expected LetDecl, got {:?}", other),
    }
}

#[test]
fn test_parse_module_type_and_include() {
    let prog = parse_program("module type S = sig end;; include Foo").expect("parse_program failed");
    assert_eq!(prog.items.len(), 2);

    match &prog.items[0] {
        AstItem::ModuleTypeDecl(mt) => {
            assert_eq!(mt.name, "S");
            assert!(matches!(mt.sig, ModuleSig::Sig(_)));
        }
        other => panic!("expected ModuleTypeDecl, got {:?}", other),
    }

    match &prog.items[1] {
        AstItem::IncludeStmt(inc) => {
            assert!(matches!(inc.module_expr, ModuleExpr::Ident(ref n) if n == "Foo"));
        }
        other => panic!("expected IncludeStmt, got {:?}", other),
    }
}

#[test]
fn test_parse_function_cases() {
    let expr = parse_item("let f = function | _ -> 0 | x -> x").expect("parse_item failed");
    match expr {
        AstItem::LetDecl(ld) => match ld.value {
            Expr::Function { cases } => assert_eq!(cases.len(), 2),
            other => panic!("expected Function expr, got {:?}", other),
        },
        other => panic!("expected LetDecl, got {:?}", other),
    }
}

#[test]
fn test_tuple_list_and_bool_exprs() {
    let item = parse_item("let v = (true, false, 3) + [1; 2; 3]").expect("parse_item failed");
    match item {
        AstItem::LetDecl(ld) => match ld.value {
            Expr::Call { callee, args } => {
                assert_eq!(callee, "+");
                assert_eq!(args.len(), 2);
                assert!(matches!(args[0], Expr::Tuple(_)));
                assert!(matches!(args[1], Expr::List(_)));
            }
            other => panic!("expected Call expr, got {:?}", other),
        },
        other => panic!("expected LetDecl, got {:?}", other),
    }
}

#[test]
fn test_nested_if_expr() {
    let item = parse_item("let x = if a then if b then 1 else 2 else 3").expect("parse_item failed");
    match item {
        AstItem::LetDecl(ld) => match ld.value {
            Expr::If { cond, then_branch, else_branch } => {
                assert!(matches!(*cond, Expr::Ident(ref n) if n == "a"));
                assert!(matches!(*then_branch, Expr::If { .. }));
                assert!(matches!(*else_branch, Expr::Int(3)));
            }
            other => panic!("expected If expr, got {:?}", other),
        },
        other => panic!("expected LetDecl, got {:?}", other),
    }
}

#[test]
fn test_parser_error_on_empty_input() {
    assert!(parse_item("").is_err());
    assert!(parse_program("  ;;  ").expect("empty program should parse").items.is_empty());
}

#[test]
fn test_constructor_and_list_patterns() {
    let item = parse_item("let r = match xs with | Some(x) -> x | [] -> 0 | [a; b] -> a").expect("parse_item failed");
    match item {
        AstItem::LetDecl(ld) => match ld.value {
            Expr::Match { cases, .. } => {
                assert_eq!(cases.len(), 3);
                assert!(matches!(cases[0].pattern, Pattern::Constructor { ref name, .. } if name == "Some"));
                assert!(matches!(cases[1].pattern, Pattern::List(ref items) if items.is_empty()));
                assert!(matches!(cases[2].pattern, Pattern::List(ref items) if items.len() == 2));
            }
            other => panic!("expected Match expr, got {:?}", other),
        },
        other => panic!("expected LetDecl, got {:?}", other),
    }
}

#[test]
fn test_malformed_let_reports_position() {
    let err = parse_item("let x = )").expect_err("expected parse error");
    assert!(err.position.is_some(), "expected an error position");
    assert!(err.message.contains("unexpected token in expr"));
}

#[test]
fn test_unclosed_struct_reports_position() {
    let err = parse_item("module M = struct let x = 1").expect_err("expected parse error");
    assert!(err.position.is_some(), "expected an error position");
    assert!(err.message.contains("close `struct`"));
}

#[test]
fn test_unclosed_sig_reports_position() {
    let err = parse_item("module type S = sig val x : int").expect_err("expected parse error");
    assert!(err.position.is_some(), "expected an error position");
    assert!(err.message.contains("close `sig`"));
}

#[test]
fn test_malformed_match_reports_position() {
    let err = parse_item("let x = match y with | _ ->").expect_err("expected parse error");
    assert!(err.position.is_some(), "expected an error position");
    assert!(err.message.contains("unexpected EOF") || err.message.contains("unexpected token"));
}

#[test]
fn test_parse_awa_literals_and_float() {
    let item = parse_item("let x = (a\"Hello\", a'W', 'a', 4.2)").expect("parse_item failed");
    match item {
        AstItem::LetDecl(ld) => match ld.value {
            Expr::Tuple(items) => {
                assert!(matches!(items[0], Expr::AwaString(_)));
                assert!(matches!(items[1], Expr::AwaChar(_)));
                assert!(matches!(items[2], Expr::Char(_)));
                assert!(matches!(items[3], Expr::Float(_)));
            }
            other => panic!("expected tuple of literals, got {:?}", other),
        },
        other => panic!("expected LetDecl, got {:?}", other),
    }
}

#[test]
fn test_parse_include_functor_apply() {
    let prog = parse_program("include MakeApp(Raylib)").expect("parse_program failed");
    assert_eq!(prog.items.len(), 1);
    match &prog.items[0] {
        AstItem::IncludeStmt(inc) => {
            assert!(matches!(inc.module_expr, ModuleExpr::Apply { .. }));
        }
        other => panic!("expected IncludeStmt, got {:?}", other),
    }
}
