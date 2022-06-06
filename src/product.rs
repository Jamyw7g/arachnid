use std::collections::HashMap;

use curl::easy::Easy2;

use crate::{request::Request, response::Response};

pub enum Product {
    Request(Request),
    Item(HashMap<String, Value>),
    Finished,
}

#[derive(Debug)]
pub enum Value {
    Int(i64),
    FLT(f64),
    Str(String),
    Dat(Vec<u8>),
    Map(HashMap<String, Value>),
    Nil,
}

impl Default for Value {
    fn default() -> Self {
        Self::Nil
    }
}


pub enum MidProduct {
    Easy(Easy2<Response>),
    Request(Request),
    Ignore
}

impl Default for MidProduct {
    fn default() -> Self {
        Self::Ignore
    }
}