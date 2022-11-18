use nom::{
    bytes::complete::tag,
    character::complete::{digit1, multispace0, multispace1},
    combinator::{map, opt},
    error::VerboseError,
    multi::many0,
    sequence::{delimited, tuple},
    IResult,
};
use nom_locate::{position, LocatedSpan};

use crate::ast::*;

pub type Span<'a> = LocatedSpan<&'a str>;
type Res<'a, T> = IResult<Span<'a>, T, VerboseError<Span<'a>>>;

fn lit_i32(c: Span) -> Res<LiteralType> {
    map(
        tuple((opt(tag("-")), digit1)),
        |(sign, digits): (Option<Span>, Span)| {
            let value: i32 = digits.fragment().parse().unwrap();
            let value = if sign.is_none() { value } else { -value };
            LiteralType::LitI32(value)
        },
    )(c)
}

fn exp_lit(c: Span) -> Res<Expression> {
    map(tuple((lit_i32, position)), |(val, pos)| Expression {
        pos,
        specifics: ExpressionType::Literal(val),
    })(c)
}

fn exp_add_sub(c: Span) -> Res<Expression> {
    let (r, pos) = position(c)?;
    let (r, left) = exp_lit(r)?;
    if let (r, Some(op)) = opt(delimited(multispace0, tag("+"), multispace0))(r)? {
        let (r, right) = exp_add_sub(r)?;

        let op = match *op.fragment() {
            "+" => Operator::Plus,
            _ => panic!("unknown operator {}", op),
        };

        Ok((
            r,
            Expression {
                pos,
                specifics: ExpressionType::Binary(Box::new(left), op, Box::new(right)),
            },
        ))
    } else {
        Ok((r, left))
    }
}

fn print_statement(c: Span) -> Res<StatementType> {
    map(
        delimited(
            tuple((tag("print"), multispace1)),
            exp_add_sub,
            tuple((multispace0, tag(";"))),
        ),
        StatementType::Print,
    )(c)
}

fn statement(c: Span) -> Res<Statement> {
    map(
        tuple((
            position,
            delimited(multispace0, print_statement, multispace0),
        )),
        |(pos, stmt)| Statement {
            pos,
            specifics: stmt,
        },
    )(c)
}

pub fn code_block(c: Span) -> Res<CodeBody> {
    map(many0(statement), |statements| CodeBody { statements })(c)
}

#[cfg(test)]
mod test {
    use std::assert_matches::assert_matches;

    use super::*;

    #[test]
    fn i32_positive() {
        let (res, result) = lit_i32("123".into()).unwrap();
        assert_eq!(*res.fragment(), "");
        assert_eq!(result, LiteralType::LitI32(123));
    }

    #[test]
    fn i32_negative() {
        let (res, result) = lit_i32("-123".into()).unwrap();
        assert_eq!(*res.fragment(), "");
        assert_eq!(result, LiteralType::LitI32(-123));
    }

    #[test]
    fn add() {
        let (res, result) = exp_add_sub("1 + 3".into()).unwrap();
        assert_eq!(*res.fragment(), "");
        assert_matches!(
            result,
            Expression {
                specifics: ExpressionType::Binary(
                    box Expression {
                        specifics: ExpressionType::Literal(LiteralType::LitI32(1)),
                        ..
                    },
                    Operator::Plus,
                    box Expression {
                        specifics: ExpressionType::Literal(LiteralType::LitI32(3)),
                        ..
                    }
                ),
                ..
            }
        )
    }

    #[test]
    fn print() {
        print_statement("print 1;".into()).unwrap();
    }

    #[test]
    fn t_statement() {
        statement("          print 1;        ".into()).unwrap();
    }
}
