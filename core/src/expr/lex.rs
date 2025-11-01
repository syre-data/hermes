use super::{
    position::{Span, WithSpan},
    token::{self, Token},
};
use crate::data;
use std::iter;

#[cfg_attr(feature = "tracing", tracing::instrument(level = "trace", fields(src = %src.as_ref())))]
pub fn tokenize(src: impl AsRef<str>) -> Lex {
    let mut lexer = Lexer::new(src.as_ref());
    lexer.tokenize();
    lexer.into()
}

pub struct Lex {
    pub tokens: Vec<WithSpan<Token>>,
    pub errors: Vec<WithSpan<error::Kind>>,
}

impl Lex {
    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty() && self.errors.is_empty()
    }
}

struct Scanner<'a> {
    /// Iterator over src characters.
    iter: iter::Peekable<iter::Enumerate<std::str::Chars<'a>>>,

    /// Cursor position.
    pos: usize,

    /// If the iterator has been fully consumed.
    complete: bool,
}

impl<'a> Scanner<'a> {
    pub fn new(src: &'a str) -> Self {
        let iter = src.chars().enumerate().peekable();
        Self {
            iter,
            pos: 0,
            complete: false,
        }
    }
}

impl<'a> Scanner<'a> {
    /// Peek at the next character without consuming it.
    pub fn peek(&mut self) -> Option<&<Self as Iterator>::Item> {
        self.iter.peek().map(|(_, char)| char)
    }

    /// Consume the next character if it is equal to the expected one.
    pub fn next_if_eq(
        &mut self,
        expected: &<Self as Iterator>::Item,
    ) -> Option<<Self as Iterator>::Item> {
        if let Some((idx, char)) = self.iter.next_if(|(_, char)| expected == char) {
            #[cfg(feature = "tracing")]
            tracing::debug!(?char);

            self.pos = idx;
            Some(char)
        } else {
            if !self.complete {
                self.pos += 1;
                self.complete = true;
            }

            None
        }
    }
}

impl<'a> iter::Iterator for Scanner<'a> {
    type Item = char;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some((idx, char)) = self.iter.next() {
            #[cfg(feature = "tracing")]
            tracing::debug!(?char);

            self.pos = idx;
            Some(char)
        } else {
            if !self.complete {
                self.pos += 1;
                self.complete = true;
            }

            None
        }
    }
}

struct Lexer<'a> {
    /// Source code input.
    it: Scanner<'a>,
    tokens: Vec<WithSpan<Token>>,
    errors: Vec<WithSpan<error::Kind>>,
}

impl<'a> Lexer<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            it: Scanner::new(src),
            tokens: vec![],
            errors: vec![],
        }
    }

    pub fn tokenize(&mut self) {
        while let Some(token) = self.match_next_token() {
            match token {
                Ok(token) => self.tokens.push(token),
                Err(err) => self.errors.push(err),
            }
        }
    }

    /// Validates if the character is valid within an identifier.
    /// Valid characters are alphabetic (`a-z`, `A-Z`) and underscore (`_`).
    fn is_valid_ident_char(ch: &char) -> bool {
        ch.is_ascii_alphabetic() || *ch == '_'
    }

    /// Validates if the character is valid within a cell reference.
    /// Valid characters are alphanumeric (`a-z`, `A-Z`, `0 - 9`), exclamation (`!`), and dollar (`$`).
    fn is_valid_cell_ref_char(ch: &char) -> bool {
        ch.is_ascii_alphanumeric() || *ch == '!' || *ch == '$'
    }

    /// Validates if the character is valid within an identifier or cell reference.
    fn is_valid_ident_or_cell_ref_char(ch: &char) -> bool {
        Self::is_valid_ident_char(ch) || Self::is_valid_cell_ref_char(ch)
    }
}

impl<'a> Lexer<'a> {
    fn next_if_else(&mut self, to_match: char, matched: Token, unmatched: Token) -> Token {
        if self.it.next_if_eq(&to_match).is_some() {
            matched
        } else {
            unmatched
        }
    }

    fn next_while<F>(&mut self, predicate: F) -> Vec<char>
    where
        F: Fn(char) -> bool,
    {
        let mut chars = vec![];
        while let Some(ch) = self.it.peek() {
            if predicate(*ch) {
                let ch = self.it.next().expect("character to be present");
                chars.push(ch);
            } else {
                break;
            }
        }
        chars
    }

    fn match_next_token(&mut self) -> Option<Result<WithSpan<Token>, WithSpan<error::Kind>>> {
        self.next_while(|ch| ch == ' ' || ch == '\t' || ch == '\r' || ch == '\n');
        let Some(char) = self.it.next() else {
            return None;
        };
        let pos_start = self.it.pos;

        let token = match char {
            ':' => Ok(WithSpan::at(Token::Colon, pos_start)),
            ',' => Ok(WithSpan::at(Token::Comma, pos_start)),
            '-' => Ok(WithSpan::at(Token::Minus, pos_start)),
            '(' => Ok(WithSpan::at(Token::ParenLeft, pos_start)),
            ')' => Ok(WithSpan::at(Token::ParenRight, pos_start)),
            '%' => Ok(WithSpan::at(Token::Percent, pos_start)),
            '+' => Ok(WithSpan::at(Token::Plus, pos_start)),
            '/' => Ok(WithSpan::at(Token::SlashForward, pos_start)),

            '*' => {
                let token = self.next_if_else('*', Token::StarStar, Token::Star);
                Ok(WithSpan::new(token, pos_start, self.it.pos))
            }

            '!' => {
                let token = self.next_if_else('=', Token::BangEqual, Token::Bang);
                Ok(WithSpan::new(token, pos_start, self.it.pos))
            }

            '<' => {
                let token = self.next_if_else('=', Token::LessEqual, Token::Less);
                Ok(WithSpan::new(token, pos_start, self.it.pos))
            }

            '>' => {
                let token = self.next_if_else('=', Token::GreaterEqual, Token::Greater);
                Ok(WithSpan::new(token, pos_start, self.it.pos))
            }

            '=' => {
                let token = self.next_if_else('=', Token::EqualEqual, Token::Equal);
                Ok(WithSpan::new(token, pos_start, self.it.pos))
            }

            '\'' | '"' => {
                let value = self.next_while(|ch| ch != char).into_iter().collect();
                if let Some(ch) = self.it.next() {
                    assert_eq!(ch, char, "delimeters should match");
                    Ok(WithSpan::new(
                        Token::String {
                            value,
                            delimeter: token::StringDelimeter::from_char(ch)
                                .expect("string delimeter is valid"),
                        },
                        pos_start,
                        self.it.pos + 1,
                    ))
                } else {
                    Err(WithSpan::new(
                        error::Kind::UnterminatedString,
                        pos_start,
                        self.it.pos,
                    ))
                }
            }

            '$' => {
                let rest = self.next_while(|ch| Self::is_valid_cell_ref_char(&ch));
                let value = iter::once('$').chain(rest).collect::<String>();
                if let Some(cell) = data::CellRef::from_str(&value) {
                    Ok(WithSpan::new(
                        Token::CellRef(cell),
                        pos_start,
                        self.it.pos + 1,
                    ))
                } else {
                    Err(WithSpan::new(
                        error::Kind::InvalidCellRef,
                        pos_start,
                        self.it.pos,
                    ))
                }
            }

            char if char.is_ascii_digit() => {
                let rest = self.next_while(|ch| ch.is_ascii_digit() || ch == '.');
                let value = iter::once(char).chain(rest).collect::<String>();

                if value
                    .chars()
                    .last()
                    .expect("at least one character in value")
                    == '.'
                {
                    Err(WithSpan::new(
                        error::Kind::RadixTerminator,
                        pos_start,
                        self.it.pos,
                    ))
                } else if value.chars().filter(|ch| *ch == '.').count() > 1 {
                    Err(WithSpan::new(
                        error::Kind::MultipleRadixPoints,
                        pos_start,
                        self.it.pos,
                    ))
                } else {
                    Ok(WithSpan::new(Token::Number(value), pos_start, self.it.pos))
                }
            }

            char if char.is_ascii_alphabetic() => {
                let rest = self.next_while(|ch| Self::is_valid_ident_or_cell_ref_char(&ch));
                let value = iter::once(char).chain(rest).collect::<String>();
                if let Some(cell) = data::CellRef::from_str(&value) {
                    Ok(WithSpan::new(
                        Token::CellRef(cell),
                        pos_start,
                        self.it.pos + 1,
                    ))
                } else {
                    let value_lower = value.to_lowercase();
                    if let Some(word) = token::Keyword::from_str(&value_lower) {
                        Ok(WithSpan::new(
                            Token::Keyword(word),
                            pos_start,
                            self.it.pos + 1,
                        ))
                    } else {
                        Ok(WithSpan::new(
                            Token::Identifier(value),
                            pos_start,
                            self.it.pos + 1,
                        ))
                    }
                }
            }

            char => Ok(WithSpan::at(Token::Unknown(char), pos_start)),
        };

        Some(token)
    }
}

impl<'a> Into<Lex> for Lexer<'a> {
    fn into(self) -> Lex {
        Lex {
            tokens: self.tokens,
            errors: self.errors,
        }
    }
}

pub mod error {
    #[derive(Debug, Clone, Copy)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub enum Kind {
        /// An invalid charater was encountered.
        UnexpectedCharacter { expected: char, found: char },
        /// A string was opened, but not closed before the end of the input.
        UnterminatedString,
        /// Number contains multiple radix points.
        /// e.g. `1.2.3`
        MultipleRadixPoints,
        /// Number ends with a radix point.
        /// e.g. `123.`
        RadixTerminator,
        /// Could not parse string as a cell reference.
        InvalidCellRef,
        /// Input ended unexpectedly.
        EndOfInput,
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::utils;
    use std::assert_matches::assert_matches;

    #[test]
    fn tokenize_empty() {
        let input = "";
        let lex = tokenize(input);
        assert!(lex.is_empty());

        let input = " \t\r\n";
        let lex = tokenize(input);
        assert!(lex.is_empty());
    }

    #[test]
    fn tokenize_literal_empty_string() {
        let input = "''";
        let lex = tokenize(input);
        assert_eq!(lex.tokens.len(), 1);
        let token = &lex.tokens[0];
        assert_eq!(token.span, Span::new(0, 2));
        let Token::String { value, delimeter } = &token.value else {
            panic!("incorrect token kind")
        };
        assert_eq!(value, "");
        assert_matches!(delimeter, token::StringDelimeter::QuoteSingle);

        let input = "\"\"";
        let lex = tokenize(input);
        assert_eq!(lex.tokens.len(), 1);
        let token = &lex.tokens[0];
        assert_eq!(token.span, Span::new(0, 2));
        let Token::String { value, delimeter } = &token.value else {
            panic!("incorrect token kind")
        };
        assert_eq!(value, "");
        assert_matches!(delimeter, token::StringDelimeter::QuoteDouble);
    }

    #[test]
    fn tokenize_literal_string() {
        let content = "test";
        let input = format!("'{content}'");
        let lex = tokenize(&input);
        assert_eq!(lex.tokens.len(), 1);
        assert!(lex.errors.is_empty());
        let token = &lex.tokens[0];
        assert_eq!(token.span, Span::new(0, 6));
        let Token::String { value, delimeter } = &token.value else {
            panic!("incorrect token kind")
        };
        assert_eq!(value, content);
        assert_matches!(delimeter, token::StringDelimeter::QuoteSingle);

        let content = "test";
        let input = format!("\"{content}\"");
        let lex = tokenize(&input);
        assert_eq!(lex.tokens.len(), 1);
        assert!(lex.errors.is_empty());
        let token = &lex.tokens[0];
        assert_eq!(token.span, Span::new(0, 6));
        let Token::String { value, delimeter } = &token.value else {
            panic!("incorrect token kind")
        };
        assert_eq!(value, content);
        assert_matches!(delimeter, token::StringDelimeter::QuoteDouble);
    }

    #[test]
    fn tokenize_string_unclosed() {
        let input = "'";
        let lex = tokenize(input);
        assert!(lex.tokens.is_empty());
        assert_eq!(lex.errors.len(), 1);
        let err = &lex.errors[0];
        assert_eq!(err.span, Span::new(0, 1));
        assert_matches!(err.value, error::Kind::UnterminatedString);

        let input = "\"";
        let lex = tokenize(input);
        assert!(lex.tokens.is_empty());
        assert_eq!(lex.errors.len(), 1);
        let err = &lex.errors[0];
        assert_eq!(err.span, Span::new(0, 1));
        assert_matches!(err.value, error::Kind::UnterminatedString);

        let input = "'test";
        let lex = tokenize(input);
        assert!(lex.tokens.is_empty());
        assert_eq!(lex.errors.len(), 1);
        let err = &lex.errors[0];
        assert_eq!(err.span, Span::new(0, 5));
        assert_matches!(err.value, error::Kind::UnterminatedString);

        let input = "\"test";
        let lex = tokenize(input);
        assert!(lex.tokens.is_empty());
        assert_eq!(lex.errors.len(), 1);
        let err = &lex.errors[0];
        assert_eq!(err.span, Span::new(0, 5));
        assert_matches!(err.value, error::Kind::UnterminatedString);
    }

    #[test]
    fn tokenize_number() {
        let input = "3";
        let lex = tokenize(input);
        assert_eq!(lex.tokens.len(), 1);
        assert!(lex.errors.is_empty());
        let token = &lex.tokens[0];
        assert_eq!(token.value, Token::Number("3".to_string()));

        let input = "3.0";
        let lex = tokenize(input);
        assert_eq!(lex.tokens.len(), 1);
        assert!(lex.errors.is_empty());
        let token = &lex.tokens[0];
        assert_eq!(token.value, Token::Number("3.0".to_string()));
    }

    #[test]
    fn tokenize_number_multiple_radix_points() {
        let input = "1..0";
        let lex = tokenize(input);
        assert_eq!(lex.errors.len(), 1);
        let err = &lex.errors[0];
        assert_matches!(err.value, error::Kind::MultipleRadixPoints);

        let input = "1.2.3";
        let lex = tokenize(input);
        assert!(lex.tokens.is_empty());
        assert_eq!(lex.errors.len(), 1);
        let err = &lex.errors[0];
        assert_matches!(err.value, error::Kind::MultipleRadixPoints);
    }

    #[test]
    fn tokenize_number_radix_terminator() {
        let input = "123.";
        let lex = tokenize(input);
        assert!(lex.tokens.is_empty());
        assert_eq!(lex.errors.len(), 1);
        let err = &lex.errors[0];
        assert_matches!(err.value, error::Kind::RadixTerminator);

        let input = "123. ";
        let lex = tokenize(input);
        assert_eq!(lex.errors.len(), 1);
        let err = &lex.errors[0];
        assert_matches!(err.value, error::Kind::RadixTerminator);
    }

    #[test]
    fn tokenize_cell_index() {
        let col = 0;
        let row = 0;
        let input = format!("{}{}", utils::index_to_col(col), utils::index_to_row(row));
        let lex = tokenize(input);
        assert_eq!(lex.tokens.len(), 1);
        assert!(lex.errors.is_empty());
        let token = &lex.tokens[0];
        assert_eq!(
            token.value,
            Token::CellRef(data::CellRef {
                sheet: data::SheetRef::Relative,
                col,
                row,
                col_mode: data::RefMode::Relative,
                row_mode: data::RefMode::Relative
            })
        );

        let col = 2;
        let row = 3;
        let input = format!("{}{}", utils::index_to_col(col), utils::index_to_row(row));
        let lex = tokenize(input);
        assert_eq!(lex.tokens.len(), 1);
        assert!(lex.errors.is_empty());
        let token = &lex.tokens[0];
        assert_eq!(
            token.value,
            Token::CellRef(data::CellRef {
                sheet: data::SheetRef::Relative,
                col,
                row,
                col_mode: data::RefMode::Relative,
                row_mode: data::RefMode::Relative
            })
        );

        let col = 30;
        let row = 40;
        let input = format!("{}{}", utils::index_to_col(col), utils::index_to_row(row));
        let lex = tokenize(input);
        assert_eq!(lex.tokens.len(), 1);
        assert!(lex.errors.is_empty());
        let token = &lex.tokens[0];
        assert_eq!(
            token.value,
            Token::CellRef(data::CellRef {
                sheet: data::SheetRef::Relative,
                col,
                row,
                col_mode: data::RefMode::Relative,
                row_mode: data::RefMode::Relative
            })
        );

        let col = 100;
        let row = 110;
        let input = format!("{}{}", utils::index_to_col(col), utils::index_to_row(row));
        let lex = tokenize(input);
        assert_eq!(lex.tokens.len(), 1);
        assert!(lex.errors.is_empty());
        let token = &lex.tokens[0];
        assert_eq!(
            token.value,
            Token::CellRef(data::CellRef {
                sheet: data::SheetRef::Relative,
                col,
                row,
                col_mode: data::RefMode::Relative,
                row_mode: data::RefMode::Relative
            })
        );
    }

    #[test]
    fn tokenize_ident() {
        let input = "a";
        let lex = tokenize(input);
        assert_eq!(lex.tokens.len(), 1);
        assert!(lex.errors.is_empty());
        let token = &lex.tokens[0];
        assert_eq!(token.value, Token::Identifier(input.to_string()));

        let input = "a_b";
        let lex = tokenize(input);
        assert_eq!(lex.tokens.len(), 1);
        assert!(lex.errors.is_empty());
        let token = &lex.tokens[0];
        assert_eq!(token.value, Token::Identifier(input.to_string()));
    }

    #[test]
    fn tokenize_keyword() {
        let reserved = [
            token::Keyword::True,
            token::Keyword::False,
            token::Keyword::And,
            token::Keyword::Or,
            token::Keyword::Sum,
        ];
        for word in reserved {
            let input = word.as_str();
            let lex = tokenize(input);
            assert_eq!(lex.tokens.len(), 1);
            assert!(lex.errors.is_empty());
            let token = &lex.tokens[0];
            let expected = Token::Keyword(word);
            assert_eq!(token.value, expected);
        }
    }
}
