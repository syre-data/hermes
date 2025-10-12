use super::{ast, lex, parse};
use crate::data;
use std::{cmp, time};

/// Provides the context to evaluate an expression in.
pub trait Context: Copy {
    /// Return the value of the cell.
    /// `None` if the cell does not exist.
    fn cell_value(&self, cell_ref: &data::CellRef) -> Option<Value>;
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Value {
    Empty,
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    DateTime(chrono::DateTime<chrono::Utc>),
    Duration(time::Duration),
}

impl Value {
    pub fn is_int(&self) -> bool {
        matches!(self, Self::Int(_))
    }

    pub fn is_float(&self) -> bool {
        matches!(self, Self::Float(_))
    }

    /// If the value is an `Int` or `Float`.
    pub fn is_number(&self) -> bool {
        self.is_int() || self.is_float()
    }

    pub fn as_int(&self) -> Option<i64> {
        if let Self::Int(value) = self {
            Some(*value)
        } else {
            None
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        if let Self::Float(value) = self {
            Some(*value)
        } else {
            None
        }
    }

    /// Value as a number.
    /// Converts `Int` to `f64` value.
    /// Does **not** attempt ot parse `String`.
    pub fn as_number(&self) -> Option<f64> {
        if let Self::Float(value) = self {
            Some(*value)
        } else if let Self::Int(value) = self {
            Some(*value as f64)
        } else {
            None
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        if let Self::Bool(value) = self {
            Some(*value)
        } else {
            None
        }
    }
}

/// Error value.
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Error {
    /// Invalid syntax.
    Tokenize(lex::error::Kind),
    /// Invalid expression.
    Parse(parse::error::Kind),
    /// Divide by 0.
    Div0,
    /// Could not parse a string as a number.
    InvalidNumber,
    /// Could not evaluate operation due to invalid arguments.
    InvalidOperation(String),
    /// Number overflow.
    Overflow,
    /// Invalid cell reference.
    InvalidCellRef(data::CellRef),
}

pub fn eval<T>(expr: ast::Expr, ctx: T) -> Result<Value, Error>
where
    T: Context,
{
    match expr {
        ast::Expr::Empty => Ok(Value::Empty),
        ast::Expr::Literal(value) => eval_literal(value, ctx),
        ast::Expr::Binary(value) => eval_binary(value, ctx),
        ast::Expr::Unary(value) => eval_unary(value, ctx),
        ast::Expr::Group(value) => eval(*value.expr, ctx),
    }
}

fn eval_literal<T>(expr: ast::ExprLiteral, ctx: T) -> Result<Value, Error>
where
    T: Context,
{
    match expr {
        ast::ExprLiteral::String(value) => Ok(Value::String(value.value)),
        ast::ExprLiteral::Bool(value) => Ok(Value::Bool(value.value)),
        ast::ExprLiteral::Number(value) => {
            let value = value.value;
            if let Ok(value) = value.parse::<i64>() {
                Ok(Value::Int(value))
            } else {
                value
                    .parse::<f64>()
                    .map(|value| Value::Float(value))
                    .map_err(|_| Error::InvalidNumber)
            }
        }
        ast::ExprLiteral::CellRef(value) => match ctx.cell_value(&value.value) {
            None => Err(Error::InvalidCellRef(value.value.clone())),
            Some(value) => Ok(value),
        },
    }
}

fn eval_binary<T>(expr: ast::ExprBinary, ctx: T) -> Result<Value, Error>
where
    T: Context,
{
    let left = eval(*expr.left, ctx)?;
    let right = eval(*expr.right, ctx)?;
    match expr.op {
        ast::OpBinary::Add => {
            if let Value::Int(left) = left
                && let Value::Int(right) = right
            {
                match left.checked_add(right) {
                    Some(value) => Ok(Value::Int(value)),
                    None => Err(Error::Overflow),
                }
            } else if left.is_number() && right.is_number() {
                let left = left.as_number().unwrap();
                let right = right.as_number().unwrap();
                Ok(Value::Float(left + right))
            } else {
                Err(Error::InvalidOperation("can only add numbers".to_string()))
            }
        }
        ast::OpBinary::Subtract => {
            if let Value::Int(left) = left
                && let Value::Int(right) = right
            {
                match left.checked_sub(right) {
                    Some(value) => Ok(Value::Int(value)),
                    None => Err(Error::Overflow),
                }
            } else if left.is_number() && right.is_number() {
                let left = left.as_number().unwrap();
                let right = right.as_number().unwrap();
                Ok(Value::Float(left - right))
            } else {
                Err(Error::InvalidOperation(
                    "can only subtract numbers".to_string(),
                ))
            }
        }
        ast::OpBinary::Multiply => {
            if let Value::Int(left) = left
                && let Value::Int(right) = right
            {
                match left.checked_mul(right) {
                    Some(value) => Ok(Value::Int(value)),
                    None => Err(Error::Overflow),
                }
            } else if left.is_number() && right.is_number() {
                let left = left.as_number().unwrap();
                let right = right.as_number().unwrap();
                Ok(Value::Float(left * right))
            } else {
                Err(Error::InvalidOperation(
                    "can only multiply numbers".to_string(),
                ))
            }
        }
        ast::OpBinary::Divide => {
            if let Value::Int(left) = left
                && let Value::Int(right) = right
            {
                if left % right == 0 {
                    match left.checked_div(right) {
                        Some(value) => Ok(Value::Int(value)),
                        None => Err(Error::Overflow),
                    }
                } else {
                    let left = left as f64;
                    let right = right as f64;
                    Ok(Value::Float(left / right))
                }
            } else if left.is_number() && right.is_number() {
                let left = left.as_number().unwrap();
                let right = right.as_number().unwrap();
                Ok(Value::Float(left / right))
            } else {
                Err(Error::InvalidOperation(
                    "can only divide numbers".to_string(),
                ))
            }
        }
        ast::OpBinary::Remainder => {
            if let Value::Int(left) = left
                && let Value::Int(right) = right
            {
                match left.checked_rem(right) {
                    Some(value) => Ok(Value::Int(value)),
                    None => Err(Error::Overflow),
                }
            } else if left.is_number() && right.is_number() {
                let left = left.as_number().unwrap();
                let right = right.as_number().unwrap();
                Ok(Value::Float(left % right))
            } else {
                Err(Error::InvalidOperation(
                    "can only take remainder numbers".to_string(),
                ))
            }
        }
        ast::OpBinary::Exp => {
            if let Value::Int(left) = left
                && let Value::Int(right) = right
            {
                if let Ok(pow) = u32::try_from(right) {
                    match left.checked_pow(pow) {
                        Some(value) => Ok(Value::Int(value)),
                        None => Err(Error::Overflow),
                    }
                } else if let Ok(pow) = i32::try_from(right) {
                    let base = left as f64;
                    Ok(Value::Float(base.powi(pow)))
                } else {
                    return Err(Error::Overflow);
                }
            } else if left.is_number()
                && let Value::Int(right) = right
            {
                let left = left.as_number().unwrap();
                let Ok(pow) = i32::try_from(right) else {
                    return Err(Error::Overflow);
                };
                Ok(Value::Float(left.powi(pow)))
            } else if left.is_number()
                && let Value::Float(right) = right
            {
                let left = left.as_number().unwrap();
                Ok(Value::Float(left.powf(right)))
            } else {
                Err(Error::InvalidOperation(
                    "can only exponentiate numbers".to_string(),
                ))
            }
        }
        ast::OpBinary::Equal => match value_eq(&left, &right) {
            Some(value) => Ok(Value::Bool(value)),
            None => Err(Error::InvalidOperation("can not compare types".to_string())),
        },
        ast::OpBinary::NotEqual => match value_eq(&left, &right) {
            Some(value) => Ok(Value::Bool(!value)),
            None => Err(Error::InvalidOperation("can not compare types".to_string())),
        },
        ast::OpBinary::Greater => match value_ord(&left, &right) {
            Some(ord) => Ok(Value::Bool(matches!(ord, cmp::Ordering::Greater))),
            None => Err(Error::InvalidOperation("can not compare types".to_string())),
        },
        ast::OpBinary::GreaterEqual => match value_ord(&left, &right) {
            Some(ord) => Ok(Value::Bool(matches!(
                ord,
                cmp::Ordering::Greater | cmp::Ordering::Equal
            ))),
            None => Err(Error::InvalidOperation("can not compare types".to_string())),
        },
        ast::OpBinary::Less => match value_ord(&left, &right) {
            Some(ord) => Ok(Value::Bool(matches!(ord, cmp::Ordering::Less))),
            None => Err(Error::InvalidOperation("can not compare types".to_string())),
        },
        ast::OpBinary::LessEqual => match value_ord(&left, &right) {
            Some(ord) => Ok(Value::Bool(matches!(
                ord,
                cmp::Ordering::Less | cmp::Ordering::Equal
            ))),
            None => Err(Error::InvalidOperation("can not compare types".to_string())),
        },
        ast::OpBinary::And => todo!(),
        ast::OpBinary::Or => todo!(),
    }
}

/// Compare two values for equality.
/// `Int` and `Float` are compared as values.
/// `None` if types can not be compared.
fn value_eq(left: &Value, right: &Value) -> Option<bool> {
    match (left, right) {
        (Value::Empty, Value::Empty) => Some(true),
        (Value::String(left), Value::String(right)) => Some(left == right),
        (Value::Bool(left), Value::Bool(right)) => Some(left == right),
        (Value::Int(left), Value::Int(right)) => Some(left == right),
        (Value::Float(left), Value::Float(right)) => Some(left == right),
        (Value::Float(left), Value::Int(right)) => Some(*left == (*right as f64)),
        (Value::Int(left), Value::Float(right)) => Some((*left as f64) == *right),
        (Value::DateTime(left), Value::DateTime(right)) => Some(left == right),
        (Value::Duration(left), Value::Duration(right)) => Some(left == right),
        _ => None,
    }
}

/// Compare two vlaues for ordering.
/// `Int`` and `Float` are compared as values.
/// If `Float` is `NaN` returns `None`.
/// `String` is compared lexicographically.
/// `None` if the types can't be ordered.
fn value_ord(left: &Value, right: &Value) -> Option<cmp::Ordering> {
    match (left, right) {
        (Value::String(left), Value::String(right)) => Some(left.cmp(right)),
        (Value::Int(left), Value::Int(right)) => Some(left.cmp(right)),
        (Value::Float(left), Value::Float(right)) => {
            if left.is_nan() || right.is_nan() {
                None
            } else if *left == 0.0 && *right == 0.0 {
                Some(cmp::Ordering::Equal)
            } else {
                Some(left.total_cmp(right))
            }
        }
        (Value::Int(left), Value::Float(right)) => {
            if right.is_nan() {
                None
            } else {
                let left = *left as f64;
                Some(left.total_cmp(&right))
            }
        }
        (Value::Float(left), Value::Int(right)) => {
            if left.is_nan() {
                None
            } else {
                let right = *right as f64;
                Some(left.total_cmp(&right))
            }
        }
        (Value::DateTime(left), Value::DateTime(right)) => Some(left.cmp(right)),
        (Value::Duration(left), Value::Duration(right)) => Some(left.cmp(right)),
        _ => None,
    }
}

fn eval_unary<T>(expr: ast::ExprUnary, ctx: T) -> Result<Value, Error>
where
    T: Context,
{
    let value = eval(*expr.expr, ctx)?;
    match expr.op {
        ast::OpUnary::Not => {
            let Value::Bool(value) = value else {
                return Err(Error::InvalidOperation(
                    "can not perform logical operations on non-boolean values".to_string(),
                ));
            };
            Ok(Value::Bool(!value))
        }
        ast::OpUnary::Minus => {
            if let Value::Float(value) = value {
                Ok(Value::Float(-value))
            } else if let Value::Int(value) = value {
                Ok(Value::Int(-value))
            } else {
                Err(Error::InvalidOperation(
                    "can not subtract non-numeric types".to_string(),
                ))
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[derive(Clone, Copy)]
    struct CtxEmpty;
    impl Context for CtxEmpty {
        fn cell_value(&self, cell_ref: &data::CellRef) -> Option<Value> {
            None
        }
    }

    #[test]
    fn eval_literal_test() {
        let ctx = CtxEmpty;

        // string
        let src = "'hi'";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::String("hi".into()));

        // int
        let src = "5";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Int(5));

        // float
        let src = "5.0";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Float(5.0));

        // bool
        let src = "true";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Bool(true));

        let src = "false";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Bool(false));
    }

    #[test]
    fn eval_cell_ref() {
        #[derive(Clone, Copy)]
        struct Ctx;
        impl Context for Ctx {
            fn cell_value(&self, cell_ref: &data::CellRef) -> Option<Value> {
                if cell_ref.sheet
                    == data::SheetRef::Absolute(data::SheetIndex::Label("string".to_string()))
                {
                    let idx = data::CellIndex::new(cell_ref.row, cell_ref.col);
                    return Some(Value::String(idx.to_string()));
                }
                if cell_ref.sheet
                    == data::SheetRef::Absolute(data::SheetIndex::Label("int".to_string()))
                {
                    return Some(Value::Int(cell_ref.row.into()));
                }
                if cell_ref.sheet
                    == data::SheetRef::Absolute(data::SheetIndex::Label("float".to_string()))
                {
                    return Some(Value::Float(cell_ref.row.into()));
                }
                if cell_ref.sheet
                    == data::SheetRef::Absolute(data::SheetIndex::Label("bool".to_string()))
                {
                    let value = cell_ref.row % 2 == 0;
                    return Some(Value::Bool(value));
                }

                None
            }
        }

        let ctx = Ctx;

        let src = "string!A1";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::String("A1".to_string()));

        let src = "int!A1";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Int(0));

        let src = "float!A1";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Float(0.0));
    }

    #[test]
    fn eval_arithmatic() {
        let ctx = CtxEmpty;

        // + (int, int)
        let src = "4 + 3";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Int(7));

        // + (int, float)
        let src = "4 + 3.0";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Float(7.0));

        let src = "4 + 3.5";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Float(7.5));

        // + (float, float)
        let src = "4.1 + 3.2";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Float(7.3));

        let src = "4.5 + 3.5";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Float(8.0));

        // - (int, int)
        let src = "4 - 3";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Int(1));

        let src = "4 - 5";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Int(-1));

        // - (int, float)
        let src = "4 - 3.0";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Float(1.0));

        let src = "4 - 4.5";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Float(-0.5));

        // - (float, float)
        let src = "4.3 - 3.2";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        // TODO: Fails on float equality. Look into arbitrary precision arithmatic.
        assert_eq!(res, Value::Float(1.1));

        let src = "4.5 - 5.5";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Float(-1.0));

        // * (int, int)
        let src = "4 * 3";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Int(12));

        let src = "-4 * 3";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Int(-12));

        // * (int, float)
        let src = "4 * 3.5";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Float(14.0));

        let src = "-4.5 * 3";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Float(-13.5));

        // * (float, float)
        let src = "4.0 * 3.0";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Float(12.0));

        let src = "-4.5 * 3.0";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Float(-13.5));

        // / (int, int)
        let src = "12 / 2";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Int(6));

        let src = "-12 / 3";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Int(-4));

        let src = "-12 / 8";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Float(-1.5));

        // / (float, int)
        let src = "12 / 2.0";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Float(6.0));

        let src = "-12.0 / 3";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Float(-4.0));

        let src = "-12 / 2.5";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Float(-4.8));

        // / (float, float)
        let src = "12.0 / 2.0";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Float(6.0));

        let src = "-12.0 / 2.5";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Float(-4.8));

        // % (int, int)
        let src = "12 % 2";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Int(0));

        let src = "12 % 5";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Int(2));

        let src = "12 % -5";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Int(2));

        // % (float, int)
        let src = "12 % 3.5";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Float(1.5));

        let src = "12.5 % 5";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Float(2.5));

        let src = "-12.0 % 5";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Float(-2.0));

        // % (float, float)
        let src = "12.5 % 2.0";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Float(0.5));

        let src = "-12.0 % 3.5";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Float(-1.5));

        // ** (int, int)
        let src = "2 ** 2";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Int(4));

        let src = "2 ** 5";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Int(32));

        let src = "2 ** -1";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Float(0.5));

        // ** (float, int)
        let src = "2 ** 1.5";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Float(2.8284271247461900976033774484194));

        let src = "2.5 ** 2";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Float(6.25));

        let src = "-2.0 ** 2";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Float(4.0));

        // ** (float, float)
        let src = "2.0 ** 2.0";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Float(4.0));

        let src = "-2.0 ** 2.0";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Float(4.0));
    }

    #[test]
    fn eval_comparison() {
        let ctx = CtxEmpty;

        // == (int, int)
        let src = "2 == 2";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Bool(true));

        let src = "1 == 2";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Bool(false));

        // == (float, int)
        let src = "2.0 == 2";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Bool(true));

        let src = "1.0 == 2";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Bool(false));

        // == (float, float)
        let src = "2.0 == 2.0";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Bool(true));

        let src = "1.0 == 2.0";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Bool(false));

        // != (int, int)
        let src = "1 != 2";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Bool(true));

        let src = "2 != 2";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Bool(false));

        // != (float, int)
        let src = "2.1 != 2";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Bool(true));

        let src = "2.0 != 2";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Bool(false));

        // != (float, float)
        let src = "2.5 != 2.0";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Bool(true));

        let src = "2.0 != 2.0";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Bool(false));

        // > (int, int)
        let src = "3 > 2";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Bool(true));

        let src = "2 > 2";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Bool(false));

        // > (float, int)
        let src = "3.0 > 2";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Bool(true));

        let src = "1 > 2.0";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Bool(false));

        // > (float, float)
        let src = "3.0 > 2.0";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Bool(true));

        let src = "1.0 > 2.0";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Bool(false));

        // >= (int, int)
        let src = "2 >= 2";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Bool(true));

        let src = "1 >= 2";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Bool(false));

        // >= (float, int)
        let src = "3.0 >= 2";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Bool(true));

        let src = "1 >= 1.0";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Bool(true));

        let src = "1 >= 2.0";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Bool(false));

        // >= (float, float)
        let src = "3.0 >= 2.0";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Bool(true));

        let src = "2.0 >= 2.0";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Bool(true));

        let src = "1.0 >= 2.0";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Bool(false));

        // < (int, int)
        let src = "1 < 2";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Bool(true));

        let src = "2 < 2";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Bool(false));

        // < (float, int)
        let src = "2 < 3.0";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Bool(true));

        let src = "2.0 < 1";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Bool(false));

        // < (float, float)
        let src = "2.0 < 3.0";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Bool(true));

        let src = "2.0 < -1.0";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Bool(false));

        // <= (int, int)
        let src = "2 <= 2";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Bool(true));

        let src = "1 <= -2";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Bool(false));

        // <= (float, int)
        let src = "-3.0 <= 2";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Bool(true));

        let src = "1 <= 1.0";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Bool(true));

        let src = "2.0 <= -1.0";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Bool(false));

        // <= (float, float)
        let src = "-3.0 <= 2.0";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Bool(true));

        let src = "-2.0 <= -2.0";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Bool(true));

        let src = "2.0 <= 1.0";
        let lex = lex::tokenize(src);
        let ast = parse::parse(&lex.tokens).expect("input to be valid");
        let Ok(res) = eval(ast, ctx) else {
            panic!("invalid input");
        };
        assert_eq!(res, Value::Bool(false));
    }
}
