use awa5_rs::*;

#[test]
fn test_keywords_and_operators() {
    let toks = Lexer::new("module type include extern let letrec fn func fun function struct end functor sig match with when as if then else").tokenize_all();

    assert!(toks.contains(&Token::Module));
    assert!(toks.contains(&Token::Type));
    assert!(toks.contains(&Token::Include));
    assert!(toks.contains(&Token::Extern));
    assert!(toks.contains(&Token::Let));
    assert!(toks.contains(&Token::LetRec));
    assert!(toks.contains(&Token::Fn));
    assert!(toks.contains(&Token::Func));
    assert!(toks.contains(&Token::Fun));
    assert!(toks.contains(&Token::Function));
    assert!(toks.contains(&Token::Struct));
    assert!(toks.contains(&Token::End));
    assert!(toks.contains(&Token::Functor));
    assert!(toks.contains(&Token::Sig));
    assert!(toks.contains(&Token::Match));
    assert!(toks.contains(&Token::With));
    assert!(toks.contains(&Token::When));
    assert!(toks.contains(&Token::As));
    assert!(toks.contains(&Token::If));
    assert!(toks.contains(&Token::Then));
    assert!(toks.contains(&Token::Else));
}

#[test]
fn test_comment_and_operator_tokenization() {
    let src = r#"
        1 + 2 - 3 * 4 / 5 % 6 == 7 != 8 < 9 <= 10 > 11 >= 12 && true || false ^ 13
        // line comment
        /* block comment */
        | => ->
    "#;

    let toks = Lexer::new(src).tokenize_all();

    assert!(toks.contains(&Token::Plus));
    assert!(toks.contains(&Token::Minus));
    assert!(toks.contains(&Token::Star));
    assert!(toks.contains(&Token::Slash));
    assert!(toks.contains(&Token::Percent));
    assert!(toks.contains(&Token::EqEq));
    assert!(toks.contains(&Token::NotEq));
    assert!(toks.contains(&Token::Lt));
    assert!(toks.contains(&Token::LtEq));
    assert!(toks.contains(&Token::Gt));
    assert!(toks.contains(&Token::GtEq));
    assert!(toks.contains(&Token::AndAnd));
    assert!(toks.contains(&Token::OrOr));
    assert!(toks.contains(&Token::Caret));
    assert!(toks.contains(&Token::Bar));
    assert!(toks.contains(&Token::FatArrow));
    assert!(toks.contains(&Token::Arrow));
    assert_eq!(toks.last().unwrap(), &Token::EOF);
}

#[test]
fn test_string_and_brackets() {
    let toks = Lexer::new(r#"["hello", (1, 2), { } ]"#).tokenize_all();
    assert!(toks.contains(&Token::LBracket));
    assert!(toks.contains(&Token::RBracket));
    assert!(toks.contains(&Token::LParen));
    assert!(toks.contains(&Token::RParen));
    assert!(toks.contains(&Token::LBrace));
    assert!(toks.contains(&Token::RBrace));
    assert!(toks.iter().any(|t| matches!(t, Token::StringLit(s) if s == "hello")));
}
