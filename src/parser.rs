use crate::ast::*;
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{digit1, multispace0, multispace1, one_of},
    combinator::{map, opt},
    error::VerboseError,
    multi::many0,
    sequence::{delimited, tuple},
    IResult,
};

type Res<'a, T> = IResult<&'a str, T, VerboseError<&'a str>>;

const VALID_STRING_CHARS: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ !?.";

fn lit_number(c: &str) -> Res<LiteralType> {
    map(
        tuple((opt(tag("-")), digit1)),
        |(sign, digits): (Option<&str>, &str)| {
            let value: i32 = digits.parse().unwrap();
            let value = if sign.is_none() { value } else { -value };
            LiteralType::Number(value)
        },
    )(c)
}

fn lit_string(c: &str) -> Res<LiteralType> {
    map(
        delimited(tag("\""), many0(one_of(VALID_STRING_CHARS)), tag("\"")),
        |st: Vec<char>| LiteralType::String(st.into_iter().collect()),
    )(c)
}

// EXPRESSION PARSER STYLE: they should parse the level below them on the left, and themself on the right

fn exp_lit(c: &str) -> Res<Expression> {
    map(alt((lit_number, lit_string)), |val| {
        Expression::Literal(val)
    })(c)
}

fn exp_paren(c: &str) -> Res<Expression> {
    alt((delimited(tag("("), exp_add_sub, tag(")")), exp_lit))(c)
}

//  We need left-ascoativity
// READ MORE: https://craftinginterpreters.com/parsing-expressions.html#the-parser-class
macro_rules! binary_expression {
    (fn $name:ident() $next:expr, {$($lit:literal => $val:expr),*}) => {
        fn $name(c: &str) -> Res<Expression> {
            let (mut r_hole, mut left) = $next(c)?;
            while let (r, Some(op)) = opt(delimited(
                multispace0,
                alt(($(tag($lit)),*)),
                multispace0,
            ))(r_hole)?
            {
                let (r, right) = $next(r)?;
                r_hole = r;
                

                let op = match op {
                    $($lit => $val),*
                    ,
                    _ => panic!("unknown operator {}", op),
                };
                left = Expression::Binary(Box::new(left), op, Box::new(right));
            } 
            Ok((r_hole, left))
        }
    };
}

binary_expression!(fn exp_mul_div() exp_paren, {
    "*" => Operator::Mul,
    "/" => Operator::Div
});

binary_expression!(fn exp_add_sub() exp_mul_div, {
    "+" => Operator::Add,
    "-" => Operator::Sub 
});


fn print_statement(c: &str) -> Res<Statement> {
    map(
        delimited(
            tuple((tag("print"), multispace1)),
            exp_add_sub,
            tuple((multispace0, tag(";"))),
        ),
        Statement::Print,
    )(c)
}

fn statement(c: &str) -> Res<Statement> {
    delimited(multispace0, print_statement, multispace0)(c)
}

pub fn code_block(c: &str) -> Res<CodeBody> {
    map(many0(statement), |statements| CodeBody { statements })(c)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn i32_positive() {
        let (res, result) = lit_number("123".into()).unwrap();
        assert_eq!(res, "");
        assert_eq!(result, LiteralType::Number(123));
    }

    #[test]
    fn i32_negative() {
        let (res, result) = lit_number("-123".into()).unwrap();
        assert_eq!(res, "");
        assert_eq!(result, LiteralType::Number(-123));
    }

    #[test]
    fn add() {
        let (res, result) = exp_add_sub("1 + 3".into()).unwrap();
        assert_eq!(res, "");
        assert_eq!(
            result,
            Expression::Binary(
                Box::new(Expression::Literal(LiteralType::Number(1))),
                Operator::Add,
                Box::new(Expression::Literal(LiteralType::Number(3))),
            )
        );
    }

    #[test]
    fn print() {
        print_statement("print 1;".into()).unwrap();
    }

    #[test]
    fn t_statement() {
        statement("          print 1;        ".into()).unwrap();
    }

    #[test]
    fn grouping() {
        let (res, _result) = exp_add_sub("1 - (5 + 2)".into()).unwrap();
        assert_eq!(res, "");
    }
}
