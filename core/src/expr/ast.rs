use super::token;
use crate::data;

#[derive(Debug, Clone, derive_more::From, PartialEq, Eq)]
pub enum Expr {
    Empty,
    Literal(ExprLiteral),
    Binary(ExprBinary),
    Unary(ExprUnary),
    Group(ExprGroup),
}

#[derive(derive_more::From, Clone, Debug, PartialEq, Eq)]
pub enum ExprLiteral {
    CellRef(LitCellRef),
    String(LitString),
    Bool(LitBool),
    Number(LitNumber),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LitCellRef {
    pub value: data::CellRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LitBool {
    pub value: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LitString {
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LitNumber {
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExprBinary {
    pub op: OpBinary,
    pub left: Box<Expr>,
    pub right: Box<Expr>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpBinary {
    Add,
    And,
    Divide,
    Equal,
    Exp,
    NotEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    Multiply,
    Or,
    Remainder,
    Subtract,
}

impl OpBinary {
    pub fn from_token(token: &token::Kind) -> Option<Self> {
        match token {
            token::Kind::BangEqual => Some(Self::NotEqual),
            token::Kind::EqualEqual => Some(Self::Equal),
            token::Kind::Greater => Some(Self::Greater),
            token::Kind::GreaterEqual => Some(Self::GreaterEqual),
            token::Kind::Less => Some(Self::Less),
            token::Kind::LessEqual => Some(Self::LessEqual),
            token::Kind::Minus => Some(Self::Subtract),
            token::Kind::Percent => Some(Self::Remainder),
            token::Kind::Plus => Some(Self::Add),
            token::Kind::SlashForward => Some(Self::Divide),
            token::Kind::Star => Some(Self::Multiply),
            token::Kind::StarStar => Some(Self::Exp),

            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExprUnary {
    pub op: OpUnary,
    pub expr: Box<Expr>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpUnary {
    Not,
    Minus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExprGroup {
    pub delimeter: GroupDelimeter,
    pub expr: Box<Expr>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum GroupDelimeter {
    /// `(...)`
    Parenthesis,
}
