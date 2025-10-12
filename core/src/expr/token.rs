use crate::data;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Keyword {
    True,
    False,
    And,
    Or,
    Sum,
}

impl Keyword {
    pub fn as_str(&self) -> &'static str {
        match self {
            Keyword::True => "true",
            Keyword::False => "false",
            Keyword::And => "and",
            Keyword::Or => "or",
            Keyword::Sum => "sum",
        }
    }

    pub fn from_str(value: impl AsRef<str>) -> Option<Self> {
        match value.as_ref() {
            "true" => Some(Self::True),
            "false" => Some(Self::False),
            "and" => Some(Self::And),
            "or" => Some(Self::Or),
            "sum" => Some(Self::Sum),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Bang,
    BangEqual,
    CellRef(data::CellRef),
    Colon,
    Comma,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Identifier(String),
    Less,
    LessEqual,
    Minus,
    Number(String),
    ParenLeft,
    ParenRight,
    Percent,
    Plus,
    Keyword(Keyword),
    SlashForward,
    Star,
    StarStar,
    String {
        value: String,
        delimeter: StringDelimeter,
    },
    Unknown(char),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringDelimeter {
    /// `'`
    QuoteSingle,
    /// `"`
    QuoteDouble,
}

impl StringDelimeter {
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            '\'' => Some(Self::QuoteSingle),
            '"' => Some(Self::QuoteDouble),
            _ => None,
        }
    }
}

/// Kind of token without any data.
/// Should match the variants in [`Token`].
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Kind {
    Bang,
    BangEqual,
    CellRef,
    Colon,
    Comma,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Identifier,
    Less,
    LessEqual,
    Minus,
    Number,
    ParenLeft,
    ParenRight,
    Percent,
    Plus,
    Keyword(Keyword),
    SlashForward,
    Star,
    StarStar,
    String,
    Unknown,
}

impl Kind {
    pub fn from_token(token: &Token) -> Self {
        match token {
            Token::Bang => Self::Bang,
            Token::BangEqual => Self::BangEqual,
            Token::Colon => Self::Colon,
            Token::Comma => Self::Comma,
            Token::Equal => Self::Equal,
            Token::EqualEqual => Self::EqualEqual,
            Token::Greater => Self::Greater,
            Token::GreaterEqual => Self::GreaterEqual,
            Token::Identifier(_) => Self::Identifier,
            Token::Less => Self::Less,
            Token::LessEqual => Self::LessEqual,
            Token::Minus => Self::Minus,
            Token::Number(_) => Self::Number,
            Token::ParenLeft => Self::ParenLeft,
            Token::ParenRight => Self::ParenRight,
            Token::Percent => Self::Percent,
            Token::Plus => Self::Plus,
            Token::CellRef { .. } => Self::CellRef,
            Token::Keyword(word) => Self::Keyword(*word),
            Token::SlashForward => Self::SlashForward,
            Token::Star => Self::Star,
            Token::StarStar => Self::StarStar,
            Token::String { .. } => Self::String,
            Token::Unknown(_) => Self::Unknown,
        }
    }
}
