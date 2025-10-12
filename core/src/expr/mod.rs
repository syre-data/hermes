//! # Inspiration
//! + [Crafting Interpreters](https://craftinginterpreters.com)
//! + [Lox in Rust](https://github.com/Darksecond/lox)
//! + [syc](https://docs.rs/syn)

mod ast;
mod eval;
mod lex;
mod parse;
mod position;
mod token;

pub use eval::{Context, Error, Value};

/// Validate the input can be parsed.
pub fn parse(input: impl AsRef<str>) -> Result<(), Error> {
    let lex = lex::tokenize(input);
    if !lex.errors.is_empty() {
        return Err(Error::Tokenize(lex.errors[0].value));
    }
    parse::parse(&lex.tokens).map_err(|err| Error::Parse(err.value))?;
    Ok(())
}

pub fn eval<T>(input: impl AsRef<str>, ctx: T) -> Result<Value, Error>
where
    T: Context,
{
    let lex = lex::tokenize(input);
    if !lex.errors.is_empty() {
        return Err(Error::Tokenize(lex.errors[0].value));
    }
    let ast = parse::parse(&lex.tokens).map_err(|err| Error::Parse(err.value))?;
    eval::eval(ast, ctx)
}
