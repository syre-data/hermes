use crate::data;
use hermes_core as core;
use serde::{Deserialize, Serialize};

pub enum Output {
    Value,
    Array,
}

#[derive(Clone)]
pub struct Formula(String);
impl Formula {
    pub fn new() -> Self {
        Self(String::new())
    }

    pub fn validate(&self) -> Result<(), error::Formula> {
        todo!()
    }

    pub fn inputs(&self) -> Result<Vec<core::data::IndexType>, error::Formula> {
        todo!()
    }

    pub fn output_kind(&self) -> Result<Output, error::Formula> {
        todo!()
    }
}

#[derive(Clone)]
pub struct Calculation {
    formula: Formula,
}

impl Calculation {
    pub fn new(formula: Formula) -> Self {
        Self { formula }
    }
}

pub mod error {
    pub enum Formula {}
}
