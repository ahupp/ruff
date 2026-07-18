use crate::{Mode, ParseOptions, parse, parse_expression, parse_module};
use ruff_python_ast::{Expr, Mod};
use ruff_text_size::{TextRange, TextSize};

#[test]
fn test_modes() {
    let source = "a[0][1][2][3][4]";

    assert!(parse(source, ParseOptions::from(Mode::Expression)).is_ok());
    assert!(parse(source, ParseOptions::from(Mode::Module)).is_ok());
}

fn parse_unit_expression(source: &str) -> Expr {
    let parsed = parse(
        source,
        ParseOptions::from(Mode::Expression).with_unit_syntax(true),
    )
    .unwrap();
    let Mod::Expression(expression) = parsed.into_syntax() else {
        unreachable!();
    };
    *expression.body
}

#[test]
fn unit_syntax_is_opt_in() {
    assert!(parse_expression("5 mm").is_err());
    assert!(matches!(parse_unit_expression("5 mm"), Expr::Call(_)));
}

#[test]
fn unit_application_lowers_to_a_reserved_call() {
    let Expr::Call(call) = parse_unit_expression("5 mm") else {
        panic!("expected the unit marker call");
    };
    assert_eq!(
        call.range,
        TextRange::new(TextSize::new(0), TextSize::new(4))
    );
    assert_eq!(call.arguments.args.len(), 2);
    assert!(matches!(call.arguments.args[0], Expr::NumberLiteral(_)));
    let Expr::StringLiteral(unit) = &call.arguments.args[1] else {
        panic!("expected the unit expression string");
    };
    assert_eq!(unit.value.to_str(), "mm");
    assert_eq!(
        unit.range,
        TextRange::new(TextSize::new(2), TextSize::new(4))
    );
}

#[test]
fn unit_syntax_accepts_upstream_simple_expression_forms() {
    for source in [
        "x parsec",
        "y.z watts",
        "area[index] meters**2",
        "[1.0, 37.0] newton meters",
        "-x dBm",
        "x**2 meters",
        "(1 inch) mm",
        "f(5 mm)",
        "[x meters for x in range(10)]",
        "x newton meters/(second*kg)",
        "x newton/(second # a comment\n *kg)",
    ] {
        parse(
            source,
            ParseOptions::from(Mode::Expression).with_unit_syntax(true),
        )
        .unwrap_or_else(|error| panic!("failed to parse `{source}`: {error}"));
    }
}

#[test]
fn unit_syntax_rejects_unparenthesized_operator_mixing() {
    for source in ["5 mm + x", "x + 5 mm"] {
        assert!(
            parse(
                source,
                ParseOptions::from(Mode::Expression).with_unit_syntax(true),
            )
            .is_err(),
            "`{source}` should require parentheses"
        );
    }
    parse(
        "(5 mm) + x",
        ParseOptions::from(Mode::Expression).with_unit_syntax(true),
    )
    .expect("parenthesized unit application should compose with operators");
}

#[test]
fn test_expr_mode_invalid_syntax1() {
    let source = "first second";
    let error = parse_expression(source).unwrap_err();

    insta::assert_debug_snapshot!(error);
}

#[test]
fn test_expr_mode_invalid_syntax2() {
    let source = r"first

second
";
    let error = parse_expression(source).unwrap_err();

    insta::assert_debug_snapshot!(error);
}

#[test]
fn test_expr_mode_invalid_syntax3() {
    let source = r"first

second

third
";
    let error = parse_expression(source).unwrap_err();

    insta::assert_debug_snapshot!(error);
}

#[test]
fn test_expr_mode_valid_syntax() {
    let source = "first

";
    let parsed = parse_expression(source).unwrap();

    insta::assert_debug_snapshot!(parsed.expr());
}

#[test]
fn test_unicode_aliases() {
    // https://github.com/RustPython/RustPython/issues/4566
    let source = r#"x = "\N{BACKSPACE}another cool trick""#;
    let suite = parse_module(source).unwrap().into_suite();

    insta::assert_debug_snapshot!(suite);
}

#[test]
fn test_ipython_escape_commands() {
    let parsed = parse(
        r"
# Normal Python code
(
    a
    %
    b
)

# Dynamic object info
??a.foo
?a.foo
?a.foo?
??a.foo()??

# Line magic
%timeit a = b
%timeit foo(b) % 3
%alias showPath pwd && ls -a
%timeit a =\
  foo(b); b = 2
%matplotlib --inline
%matplotlib \
    --inline

# System shell access
!pwd && ls -a | sed 's/^/\    /'
!pwd \
  && ls -a | sed 's/^/\\    /'
!!cd /Users/foo/Library/Application\ Support/

# Let's add some Python code to make sure that earlier escapes were handled
# correctly and that we didn't consume any of the following code as a result
# of the escapes.
def foo():
    return (
        a
        !=
        b
    )

# Transforms into `foo(..)`
/foo 1 2
;foo 1 2
,foo 1 2

# Indented escape commands
for a in range(5):
    !ls

p1 = !pwd
p2: str = !pwd
foo = %foo \
    bar

% foo
foo = %foo  # comment

# Help end line magics
foo?
foo.bar??
foo.bar.baz?
foo[0]??
foo[0][1]?
foo.bar[0].baz[1]??
foo.bar[0].baz[2].egg??
"
        .trim(),
        ParseOptions::from(Mode::Ipython),
    )
    .unwrap();
    insta::assert_debug_snapshot!(parsed.syntax());
}

#[test]
fn test_fstring_expr_inner_line_continuation_and_t_string() {
    let source = r#"f'{\t"i}'"#;

    let parsed = parse_expression(source);

    let error = parsed.unwrap_err();

    insta::assert_debug_snapshot!(error);
}

#[test]
fn test_fstring_expr_inner_line_continuation_newline_t_string() {
    let source = r#"f'{\
t"i}'"#;

    let parsed = parse_expression(source);

    let error = parsed.unwrap_err();

    insta::assert_debug_snapshot!(error);
}
