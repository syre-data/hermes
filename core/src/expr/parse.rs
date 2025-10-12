use super::{
    ast, lex,
    position::WithSpan,
    token::{self, Token},
};
use crate::data;
use std::{assert_matches::assert_matches, iter};

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum Precedence {
    None,
    /// ||
    Or,
    /// &&
    And,
    /// == != <= >= < >
    Compare,
    /// + -
    Sum,
    /// * / %
    Product,
    /// **
    Exponent,
    /// ! -
    Prefix,
    /// Functions
    Unambiguous,
}

impl Precedence {
    pub fn of(token: &token::Kind) -> Self {
        match token {
            token::Kind::ParenRight => Self::None,

            token::Kind::Bang
            | token::Kind::BangEqual
            | token::Kind::EqualEqual
            | token::Kind::Greater
            | token::Kind::GreaterEqual
            | token::Kind::Less
            | token::Kind::LessEqual => Self::Compare,

            token::Kind::Plus | token::Kind::Minus => Self::Sum,

            token::Kind::Star | token::Kind::SlashForward | token::Kind::Percent => Self::Product,

            token::Kind::StarStar => Self::Exponent,

            token::Kind::Keyword(keyword) => todo!(),

            token::Kind::Equal => Self::Prefix,

            token::Kind::CellRef
            | token::Kind::Colon
            | token::Kind::Comma
            | token::Kind::Identifier
            | token::Kind::Number
            | token::Kind::ParenLeft
            | token::Kind::String => Self::Unambiguous,

            token::Kind::Unknown => todo!(),
        }
    }
}

struct Parser<'a> {
    tokens: &'a [WithSpan<Token>],
    cursor: usize,
    errors: Vec<WithSpan<error::Kind>>,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a Vec<WithSpan<Token>>) -> Self {
        Self {
            tokens,
            cursor: 0,
            errors: vec![],
        }
    }

    pub fn idx(&self) -> usize {
        self.cursor
    }
}

impl<'a> Parser<'a> {
    /// # Returns
    /// If the parser has reached the end of the tokens.
    pub fn eof(&self) -> bool {
        self.cursor >= self.tokens.len()
    }

    pub fn peek(&mut self) -> Option<token::Kind> {
        self.tokens
            .get(self.cursor)
            .map(|token| token::Kind::from_token(&token.value))
    }
}

impl<'a> Iterator for Parser<'a> {
    type Item = &'a WithSpan<Token>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.eof() {
            None
        } else {
            let next = self.tokens.get(self.cursor);
            self.cursor += 1;
            next
        }
    }
}

pub fn parse(tokens: &Vec<WithSpan<Token>>) -> Result<ast::Expr, WithSpan<error::Kind>> {
    if tokens.is_empty() {
        return Ok(ast::Expr::Empty);
    }

    let mut parser = Parser::new(tokens);
    parse_expr(&mut parser, Precedence::None)
}

#[cfg_attr(feature = "tracing", tracing::instrument(level = "trace", skip_all))]
fn parse_expr<'a>(
    parser: &mut Parser<'a>,
    precedence: Precedence,
) -> Result<ast::Expr, WithSpan<error::Kind>> {
    let mut expr = parse_prefix(parser)?;
    while !parser.eof() {
        let next = parser.peek().expect("tokens still exist");
        #[cfg(feature = "tracing")]
        tracing::debug!(?next);

        if precedence >= Precedence::of(&next) {
            break;
        }
        expr = parse_infix(parser, expr)?;
    }
    Ok(expr)
}

fn parse_prefix<'a>(parser: &mut Parser<'a>) -> Result<ast::Expr, WithSpan<error::Kind>> {
    static VALID_PREFIX_TOKENS: &'static [token::Kind] = &[
        token::Kind::Bang,
        token::Kind::CellRef,
        token::Kind::Identifier,
        token::Kind::Minus,
        token::Kind::Number,
        token::Kind::ParenLeft,
        token::Kind::Keyword(token::Keyword::True),
        token::Kind::Keyword(token::Keyword::False),
        token::Kind::Keyword(token::Keyword::Sum),
        token::Kind::String,
    ];

    let Some(token) = parser.peek() else {
        return Err(WithSpan::at(
            error::Kind::UnexpectedEndOfInut,
            parser.cursor,
        ));
    };

    match token {
        token::Kind::String | token::Kind::CellRef | token::Kind::Number => {
            Ok(parse_literal(parser)?.into())
        }
        token::Kind::Bang | token::Kind::Minus => parse_unary(parser),
        token::Kind::BangEqual
        | token::Kind::Colon
        | token::Kind::Comma
        | token::Kind::Equal
        | token::Kind::EqualEqual
        | token::Kind::Greater
        | token::Kind::GreaterEqual
        | token::Kind::Less
        | token::Kind::LessEqual
        | token::Kind::Percent
        | token::Kind::Plus
        | token::Kind::SlashForward
        | token::Kind::Star
        | token::Kind::StarStar => {
            let idx = parser.idx();
            Err(WithSpan::at(error::Kind::InvalidPrefix, idx))
        }
        token::Kind::Keyword(word) => match word {
            token::Keyword::True | token::Keyword::False => Ok(parse_literal(parser)?.into()),
            token::Keyword::And => todo!(),
            token::Keyword::Or => todo!(),
            token::Keyword::Sum => todo!(),
        },
        token::Kind::Identifier => todo!(),
        token::Kind::ParenLeft => Ok(parse_group(parser)?.into()),
        token::Kind::ParenRight => Err(WithSpan::at(
            error::Kind::UnexpectedToken {
                expected: VALID_PREFIX_TOKENS.to_vec(),
                found: token::Kind::ParenRight,
            },
            parser.cursor,
        )),
        token::Kind::Unknown => todo!(),
    }
}

fn parse_infix<'a>(
    parser: &mut Parser<'a>,
    lhs: ast::Expr,
) -> Result<ast::Expr, WithSpan<error::Kind>> {
    static VALID_TOKEN_KINDS: &[token::Kind] = &[
        token::Kind::Bang,
        token::Kind::BangEqual,
        token::Kind::EqualEqual,
        token::Kind::Greater,
        token::Kind::GreaterEqual,
        token::Kind::Less,
        token::Kind::LessEqual,
        token::Kind::Minus,
        token::Kind::ParenLeft,
        token::Kind::ParenRight,
        token::Kind::Percent,
        token::Kind::Plus,
        token::Kind::SlashForward,
        token::Kind::Star,
        token::Kind::StarStar,
    ];

    let next = parser.peek().expect("tokens still exist");
    match next {
        token::Kind::BangEqual
        | token::Kind::EqualEqual
        | token::Kind::Greater
        | token::Kind::GreaterEqual
        | token::Kind::Less
        | token::Kind::LessEqual
        | token::Kind::Minus
        | token::Kind::Percent
        | token::Kind::Plus
        | token::Kind::SlashForward
        | token::Kind::Star
        | token::Kind::StarStar => Ok(parse_binary(parser, lhs)?.into()),

        token::Kind::ParenLeft => todo!(),
        token::Kind::ParenRight => todo!(),
        token::Kind::Keyword(word) => match word {
            token::Keyword::And => todo!(),
            token::Keyword::Or => todo!(),
            token::Keyword::Sum => todo!(),
            keyword => {
                return Err(WithSpan::at(
                    error::Kind::UnexpectedToken {
                        expected: VALID_TOKEN_KINDS.to_vec(),
                        found: token::Kind::Keyword(keyword),
                    },
                    parser.cursor,
                ));
            }
        },
        token => {
            return Err(WithSpan::at(
                error::Kind::UnexpectedToken {
                    expected: VALID_TOKEN_KINDS.to_vec(),
                    found: token,
                },
                parser.cursor,
            ));
        }
    }
}

#[cfg_attr(feature = "tracing", tracing::instrument(level = "trace", skip_all))]
fn parse_group<'a>(parser: &mut Parser<'a>) -> Result<ast::ExprGroup, WithSpan<error::Kind>> {
    let pos_start = parser.cursor;
    let open_delimeter = parser.next().expect("tokens to exist");
    let expr = match parse_expr(parser, Precedence::None) {
        Ok(expr) => expr,
        Err(err) => match err.value {
            error::Kind::UnexpectedToken { found, .. } => {
                if open_delimeter.value == Token::ParenLeft
                    && matches!(found, token::Kind::ParenRight)
                {
                    ast::Expr::Empty
                } else {
                    return Err(err);
                }
            }
            _ => return Err(err),
        },
    };
    let Some(close_delimeter) = parser.next() else {
        return Err(WithSpan::new(
            error::Kind::UnexpectedEndOfInut,
            pos_start,
            parser.cursor,
        ));
    };

    if open_delimeter.value == Token::ParenLeft && close_delimeter.value == Token::ParenRight {
        Ok(ast::ExprGroup {
            delimeter: ast::GroupDelimeter::Parenthesis,
            expr: Box::new(expr),
        })
    } else {
        Err(WithSpan::new(
            error::Kind::UnclosedGroup {
                expeted: ast::GroupDelimeter::Parenthesis,
            },
            pos_start,
            parser.cursor,
        ))
    }
}

#[cfg_attr(feature = "tracing", tracing::instrument(level = "trace", skip_all))]
fn parse_binary<'a>(
    parser: &mut Parser<'a>,
    lhs: ast::Expr,
) -> Result<ast::ExprBinary, WithSpan<error::Kind>> {
    let op_token = parser.next().expect("tokens still exist");
    let op = ast::OpBinary::from_token(&token::Kind::from_token(&op_token.value))
        .expect(&format!("invalid token kind {op_token:?}"));
    let rhs = parse_expr(parser, Precedence::None)?;
    Ok(ast::ExprBinary {
        op,
        left: Box::new(lhs),
        right: Box::new(rhs),
    })
}

#[cfg_attr(feature = "tracing", tracing::instrument(level = "trace", skip_all))]
fn parse_unary<'a>(parser: &mut Parser<'a>) -> Result<ast::Expr, WithSpan<error::Kind>> {
    let next = parser.next().expect("non-empty token stream");
    #[cfg(feature = "tracing")]
    tracing::debug!(?next);

    match &next.value {
        Token::Minus => {
            let Some(token) = parser.peek() else {
                return Err(WithSpan::at(
                    error::Kind::UnexpectedEndOfInut,
                    parser.cursor,
                ));
            };

            if let token::Kind::Number = token {
                let token = parser.next().unwrap();
                let Token::Number(value) = &token.value else {
                    unreachable!();
                };

                Ok(ast::Expr::Literal(
                    ast::LitNumber {
                        value: format!("-{value}"),
                    }
                    .into(),
                ))
            } else {
                let expr = parse_expr(parser, Precedence::Prefix)?;
                Ok(ast::ExprUnary {
                    op: ast::OpUnary::Minus,
                    expr: Box::new(expr),
                }
                .into())
            }
        }

        Token::Bang => {
            let expr = parse_expr(parser, Precedence::Prefix)?;
            Ok(ast::ExprUnary {
                op: ast::OpUnary::Not,
                expr: Box::new(expr),
            }
            .into())
        }

        _ => unreachable!("invalid unary token"),
    }
}

#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
fn parse_literal<'a>(input: &mut Parser<'a>) -> Result<ast::ExprLiteral, WithSpan<error::Kind>> {
    let next = input.next().expect("non-empty token stream");
    #[cfg(feature = "tracing")]
    tracing::debug!(?next);

    match &next.value {
        Token::String { value, .. } => Ok(ast::LitString {
            value: value.clone(),
        }
        .into()),

        Token::Number(value) => Ok(ast::LitNumber {
            value: value.clone(),
        }
        .into()),

        Token::CellRef(value) => Ok(ast::LitCellRef {
            value: value.clone(),
        }
        .into()),

        Token::Keyword(word) => match word {
            token::Keyword::True => Ok(ast::LitBool { value: true }.into()),
            token::Keyword::False => Ok(ast::LitBool { value: false }.into()),
            _ => unreachable!("invalid keyword"),
        },

        _ => unreachable!("invalid literal token"),
    }
}

pub mod error {
    use super::{ast, token};
    use crate::expr::parse::ast::GroupDelimeter;

    #[derive(Debug, derive_more::From, Clone)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub enum Kind {
        /// The input was unexpectedly empty.
        UnexpectedEndOfInut,

        /// An unexpected kind of token was found.
        UnexpectedToken {
            expected: Vec<token::Kind>,
            found: token::Kind,
        },

        /// The token is not valid as a prefix.
        InvalidPrefix,

        /// A group wasn't closed.
        UnclosedGroup {
            expeted: ast::GroupDelimeter,
        },

        Binary(KindBinary),
    }

    #[derive(Debug, Clone, Copy)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub enum KindBinary {
        InvalidRhs,
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::assert_matches::assert_matches;

    #[test]
    fn parse_group() {
        // empty
        let src = "()";
        let lex = lex::tokenize(src);
        let expr = parse(&lex.tokens).expect("input should be valid");
        let ast::Expr::Group(ast::ExprGroup {
            delimeter,
            expr: inner,
        }) = expr
        else {
            panic!("invalid expression");
        };
        assert_matches!(delimeter, ast::GroupDelimeter::Parenthesis);
        assert_matches!(*inner, ast::Expr::Empty);

        // basic
        let src = "('test')";
        let lex = lex::tokenize(src);
        let expr = parse(&lex.tokens).expect("input to be valid");
        let ast::Expr::Group(ast::ExprGroup { delimeter, expr }) = expr else {
            panic!("invalid expression");
        };
        assert_matches!(delimeter, ast::GroupDelimeter::Parenthesis);
        assert_matches!(
            *expr,
            ast::Expr::Literal(ast::ExprLiteral::String(ast::LitString { .. }))
        );

        // nested
        let src = "('test' + (3 + 4))";
        let lex = lex::tokenize(src);
        let expr = parse(&lex.tokens).expect("input to be valid");
        let ast::Expr::Group(ast::ExprGroup { delimeter, expr }) = expr else {
            panic!("invalid expression");
        };
        assert_matches!(delimeter, ast::GroupDelimeter::Parenthesis);
        let ast::Expr::Binary(ast::ExprBinary { op, left, right }) = *expr else {
            panic!("invalid expression");
        };
        assert_matches!(op, ast::OpBinary::Add);
        assert_matches!(*right, ast::Expr::Group(_));

        // err: eof
        let src = "(";
        let lex = lex::tokenize(src);
        let err = parse(&lex.tokens).expect_err("input should be invalid");
        assert_matches!(err.value, error::Kind::UnexpectedEndOfInut);

        let src = "('test'";
        let lex = lex::tokenize(src);
        let err = parse(&lex.tokens).expect_err("input should be invalid");
        assert_matches!(err.value, error::Kind::UnexpectedEndOfInut);

        // err: open right
        let src = ")";
        let lex = lex::tokenize(src);
        let err = parse(&lex.tokens).expect_err("input should be invalid");
        assert_matches!(err.value, error::Kind::UnexpectedToken { .. });
    }

    #[test]
    fn parse_binary_test() {
        // +
        let src = "1 + 2";
        let lex = lex::tokenize(src);
        let expr = parse(&lex.tokens).expect("input to be valid");
        let ast::Expr::Binary(ast::ExprBinary { op, left, right }) = expr else {
            panic!("invalid expression");
        };
        assert_matches!(op, ast::OpBinary::Add);
        let ast::Expr::Literal(ast::ExprLiteral::Number(ast::LitNumber { value: left })) = *left
        else {
            panic!("invalid expression");
        };
        let ast::Expr::Literal(ast::ExprLiteral::Number(ast::LitNumber { value: right })) = *right
        else {
            panic!("invalid expression");
        };
        assert_eq!(left, "1");
        assert_eq!(right, "2");

        // -
        let src = "1 - 2";
        let lex = lex::tokenize(src);
        let expr = parse(&lex.tokens).expect("input to be valid");
        let ast::Expr::Binary(ast::ExprBinary { op, left, right }) = expr else {
            panic!("invalid expression");
        };
        assert_matches!(op, ast::OpBinary::Subtract);
        let ast::Expr::Literal(ast::ExprLiteral::Number(ast::LitNumber { value: left })) = *left
        else {
            panic!("invalid expression");
        };
        let ast::Expr::Literal(ast::ExprLiteral::Number(ast::LitNumber { value: right })) = *right
        else {
            panic!("invalid expression");
        };
        assert_eq!(left, "1");
        assert_eq!(right, "2");

        // *
        let src = "1 * 2";
        let lex = lex::tokenize(src);
        let expr = parse(&lex.tokens).expect("input to be valid");
        let ast::Expr::Binary(ast::ExprBinary { op, left, right }) = expr else {
            panic!("invalid expression");
        };
        assert_matches!(op, ast::OpBinary::Multiply);
        let ast::Expr::Literal(ast::ExprLiteral::Number(ast::LitNumber { value: left })) = *left
        else {
            panic!("invalid expression");
        };
        let ast::Expr::Literal(ast::ExprLiteral::Number(ast::LitNumber { value: right })) = *right
        else {
            panic!("invalid expression");
        };
        assert_eq!(left, "1");
        assert_eq!(right, "2");

        // /
        let src = "1 / 2";
        let lex = lex::tokenize(src);
        let expr = parse(&lex.tokens).expect("input to be valid");
        let ast::Expr::Binary(ast::ExprBinary { op, left, right }) = expr else {
            panic!("invalid expression");
        };
        assert_matches!(op, ast::OpBinary::Divide);
        let ast::Expr::Literal(ast::ExprLiteral::Number(ast::LitNumber { value: left })) = *left
        else {
            panic!("invalid expression");
        };
        let ast::Expr::Literal(ast::ExprLiteral::Number(ast::LitNumber { value: right })) = *right
        else {
            panic!("invalid expression");
        };
        assert_eq!(left, "1");
        assert_eq!(right, "2");

        // %
        let src = "1 % 2";
        let lex = lex::tokenize(src);
        let expr = parse(&lex.tokens).expect("input to be valid");
        let ast::Expr::Binary(ast::ExprBinary { op, left, right }) = expr else {
            panic!("invalid expression");
        };
        assert_matches!(op, ast::OpBinary::Remainder);
        let ast::Expr::Literal(ast::ExprLiteral::Number(ast::LitNumber { value: left })) = *left
        else {
            panic!("invalid expression");
        };
        let ast::Expr::Literal(ast::ExprLiteral::Number(ast::LitNumber { value: right })) = *right
        else {
            panic!("invalid expression");
        };
        assert_eq!(left, "1");
        assert_eq!(right, "2");

        // <
        let src = "1 < 2";
        let lex = lex::tokenize(src);
        let expr = parse(&lex.tokens).expect("input to be valid");
        let ast::Expr::Binary(ast::ExprBinary { op, left, right }) = expr else {
            panic!("invalid expression");
        };
        assert_matches!(op, ast::OpBinary::Less);
        let ast::Expr::Literal(ast::ExprLiteral::Number(ast::LitNumber { value: left })) = *left
        else {
            panic!("invalid expression");
        };
        let ast::Expr::Literal(ast::ExprLiteral::Number(ast::LitNumber { value: right })) = *right
        else {
            panic!("invalid expression");
        };
        assert_eq!(left, "1");
        assert_eq!(right, "2");

        // >
        let src = "1 > 2";
        let lex = lex::tokenize(src);
        let expr = parse(&lex.tokens).expect("input to be valid");
        let ast::Expr::Binary(ast::ExprBinary { op, left, right }) = expr else {
            panic!("invalid expression");
        };
        assert_matches!(op, ast::OpBinary::Greater);
        let ast::Expr::Literal(ast::ExprLiteral::Number(ast::LitNumber { value: left })) = *left
        else {
            panic!("invalid expression");
        };
        let ast::Expr::Literal(ast::ExprLiteral::Number(ast::LitNumber { value: right })) = *right
        else {
            panic!("invalid expression");
        };
        assert_eq!(left, "1");
        assert_eq!(right, "2");

        // <=
        let src = "1 <= 2";
        let lex = lex::tokenize(src);
        let expr = parse(&lex.tokens).expect("input to be valid");
        let ast::Expr::Binary(ast::ExprBinary { op, left, right }) = expr else {
            panic!("invalid expression");
        };
        assert_matches!(op, ast::OpBinary::LessEqual);
        let ast::Expr::Literal(ast::ExprLiteral::Number(ast::LitNumber { value: left })) = *left
        else {
            panic!("invalid expression");
        };
        let ast::Expr::Literal(ast::ExprLiteral::Number(ast::LitNumber { value: right })) = *right
        else {
            panic!("invalid expression");
        };
        assert_eq!(left, "1");
        assert_eq!(right, "2");

        // >
        let src = "1 >= 2";
        let lex = lex::tokenize(src);
        let expr = parse(&lex.tokens).expect("input to be valid");
        let ast::Expr::Binary(ast::ExprBinary { op, left, right }) = expr else {
            panic!("invalid expression");
        };
        assert_matches!(op, ast::OpBinary::GreaterEqual);
        let ast::Expr::Literal(ast::ExprLiteral::Number(ast::LitNumber { value: left })) = *left
        else {
            panic!("invalid expression");
        };
        let ast::Expr::Literal(ast::ExprLiteral::Number(ast::LitNumber { value: right })) = *right
        else {
            panic!("invalid expression");
        };
        assert_eq!(left, "1");
        assert_eq!(right, "2");

        // ==
        let src = "1 == 2";
        let lex = lex::tokenize(src);
        let expr = parse(&lex.tokens).expect("input to be valid");
        let ast::Expr::Binary(ast::ExprBinary { op, left, right }) = expr else {
            panic!("invalid expression");
        };
        assert_matches!(op, ast::OpBinary::Equal);
        let ast::Expr::Literal(ast::ExprLiteral::Number(ast::LitNumber { value: left })) = *left
        else {
            panic!("invalid expression");
        };
        let ast::Expr::Literal(ast::ExprLiteral::Number(ast::LitNumber { value: right })) = *right
        else {
            panic!("invalid expression");
        };
        assert_eq!(left, "1");
        assert_eq!(right, "2");

        // !=
        let src = "1 != 2";
        let lex = lex::tokenize(src);
        let expr = parse(&lex.tokens).expect("input to be valid");
        let ast::Expr::Binary(ast::ExprBinary { op, left, right }) = expr else {
            panic!("invalid expression");
        };
        assert_matches!(op, ast::OpBinary::NotEqual);
        let ast::Expr::Literal(ast::ExprLiteral::Number(ast::LitNumber { value: left })) = *left
        else {
            panic!("invalid expression");
        };
        let ast::Expr::Literal(ast::ExprLiteral::Number(ast::LitNumber { value: right })) = *right
        else {
            panic!("invalid expression");
        };
        assert_eq!(left, "1");
        assert_eq!(right, "2");
    }

    #[test]
    fn parse_unary_test() {
        // -
        let cell = data::CellRef {
            sheet: data::SheetRef::Relative,
            row: 0,
            col: 0,
            col_mode: data::RefMode::Relative,
            row_mode: data::RefMode::Relative,
        };
        let src = format!("-{cell}");
        let lex = lex::tokenize(src);
        let expr = parse(&lex.tokens).expect("input to be valid");

        let ast::Expr::Unary(ast::ExprUnary { op, expr: expr_num }) = expr else {
            panic!("expected unary found {expr:?}");
        };

        assert_matches!(op, ast::OpUnary::Minus,);
        assert_eq!(
            *expr_num,
            ast::Expr::Literal(ast::ExprLiteral::CellRef(ast::LitCellRef { value: cell }))
        );

        // !
        let cell = data::CellRef {
            sheet: data::SheetRef::Relative,
            row: 0,
            col: 0,
            col_mode: data::RefMode::Relative,
            row_mode: data::RefMode::Relative,
        };
        let src = format!("!{cell}");
        let lex = lex::tokenize(src);
        let expr = parse(&lex.tokens).expect("input to be valid");

        let ast::Expr::Unary(ast::ExprUnary { op, expr: expr_num }) = expr else {
            panic!("expected unary found {expr:?}",);
        };

        assert_matches!(op, ast::OpUnary::Not);
        assert_eq!(
            *expr_num,
            ast::Expr::Literal(ast::ExprLiteral::CellRef(ast::LitCellRef { value: cell }))
        );
    }

    #[test]
    fn parse_index() {
        let cell = data::CellRef {
            sheet: data::SheetRef::Relative,
            row: 0,
            col: 0,
            row_mode: data::RefMode::Relative,
            col_mode: data::RefMode::Relative,
        };
        let src = format!("{cell}");
        let lex = lex::tokenize(&src);
        assert_eq!(lex.tokens.len(), 1);
        let expr = parse(&lex.tokens).expect("input to be valid");
        let ast::Expr::Literal(ast::ExprLiteral::CellRef(ast::LitCellRef { value })) = expr else {
            panic!("invalid expression");
        };
        assert_eq!(value, cell);

        let cell = data::CellRef {
            sheet: data::SheetRef::Relative,
            row: 3,
            col: 5,
            row_mode: data::RefMode::Absolute,
            col_mode: data::RefMode::Absolute,
        };
        let src = format!("{cell}");
        let lex = lex::tokenize(&src);
        assert_eq!(lex.tokens.len(), 1);
        let expr = parse(&lex.tokens).expect("input to be valid");
        let ast::Expr::Literal(ast::ExprLiteral::CellRef(ast::LitCellRef { value })) = expr else {
            panic!("invalid expression");
        };
        assert_eq!(value, cell);

        let cell = data::CellRef {
            sheet: data::SheetIndex::Label("sheet".to_string()).into(),
            row: 2,
            col: 4,
            row_mode: data::RefMode::Relative,
            col_mode: data::RefMode::Absolute,
        };
        let src = format!("{cell}");
        let lex = lex::tokenize(&src);
        assert_eq!(lex.tokens.len(), 1);
        let expr = parse(&lex.tokens).expect("input to be valid");
        let ast::Expr::Literal(ast::ExprLiteral::CellRef(ast::LitCellRef { value })) = expr else {
            panic!("invalid expression");
        };
        assert_eq!(value, cell);
    }

    #[test]
    fn parse_lit_number() {
        let src = "3";
        let lex = lex::tokenize(src);
        assert_eq!(lex.tokens.len(), 1);
        let expr = parse(&lex.tokens).expect("input to be valid");
        let ast::Expr::Literal(ast::ExprLiteral::Number(ast::LitNumber { value })) = expr else {
            panic!("invalid expression");
        };
        assert_eq!(value, src);

        let src = "3.0123";
        let lex = lex::tokenize(src);
        assert_eq!(lex.tokens.len(), 1);
        let expr = parse(&lex.tokens).expect("input to be valid");
        let ast::Expr::Literal(ast::ExprLiteral::Number(ast::LitNumber { value })) = expr else {
            panic!("invalid expression");
        };
        assert_eq!(value, src);

        let src = "300";
        let lex = lex::tokenize(src);
        assert_eq!(lex.tokens.len(), 1);
        let expr = parse(&lex.tokens).expect("input to be valid");
        let ast::Expr::Literal(ast::ExprLiteral::Number(ast::LitNumber { value })) = expr else {
            panic!("invalid expression");
        };
        assert_eq!(value, src);

        let src = "-1";
        let lex = lex::tokenize(src);
        assert_eq!(lex.tokens.len(), 2);
        let expr = parse(&lex.tokens).expect("input to be valid");
        let ast::Expr::Literal(ast::ExprLiteral::Number(ast::LitNumber { value })) = expr else {
            panic!("invalid expression");
        };
        assert_eq!(value, src);

        let src = "-1.123";
        let lex = lex::tokenize(src);
        assert_eq!(lex.tokens.len(), 2);
        let expr = parse(&lex.tokens).expect("input to be valid");
        let ast::Expr::Literal(ast::ExprLiteral::Number(ast::LitNumber { value })) = expr else {
            panic!("invalid expression");
        };
        assert_eq!(value, src);

        let src = "-10";
        let lex = lex::tokenize(src);
        assert_eq!(lex.tokens.len(), 2);
        let expr = parse(&lex.tokens).expect("input to be valid");
        let ast::Expr::Literal(ast::ExprLiteral::Number(ast::LitNumber { value })) = expr else {
            panic!("invalid expression");
        };
        assert_eq!(value, src);
    }

    #[test]
    fn parse_lit_string() {
        let content = "test";
        let src = format!("\"{content}\"");
        let lex = lex::tokenize(&src);
        assert_eq!(lex.tokens.len(), 1);
        let expr = parse(&lex.tokens).expect("input to be valid");
        let ast::Expr::Literal(ast::ExprLiteral::String(ast::LitString { value, .. })) = expr
        else {
            panic!("invalid expression");
        };
        assert_eq!(value, content);

        let content = "test";
        let src = format!("'{content}'");
        let lex = lex::tokenize(&src);
        assert_eq!(lex.tokens.len(), 1);
        let expr = parse(&lex.tokens).expect("input to be valid");
        let ast::Expr::Literal(ast::ExprLiteral::String(ast::LitString { value, .. })) = expr
        else {
            panic!("invalid expression");
        };
        assert_eq!(value, content);
    }

    #[test]
    fn parse_lit_bool() {
        let src = "true";
        let lex = lex::tokenize(src);
        assert_eq!(lex.tokens.len(), 1);
        let expr = parse(&lex.tokens).expect("input to be valid");
        assert_matches!(
            expr,
            ast::Expr::Literal(ast::ExprLiteral::Bool(ast::LitBool { value: true }))
        );

        let src = "false";
        let lex = lex::tokenize(src);
        assert_eq!(lex.tokens.len(), 1);
        let expr = parse(&lex.tokens).expect("input to be valid");
        assert_matches!(
            expr,
            ast::Expr::Literal(ast::ExprLiteral::Bool(ast::LitBool { value: false }))
        );
    }

    #[test]
    fn parse_empty() {
        let src = "";
        let lex = lex::tokenize(src);
        let expr = parse(&lex.tokens).expect("empty token list to be valid");
        assert_matches!(expr, ast::Expr::Empty);
    }
}
