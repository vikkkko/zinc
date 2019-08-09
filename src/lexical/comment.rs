//!
//! The comment lexeme.
//!

use serde_derive::Serialize;

#[derive(Debug, Serialize, PartialEq)]
pub struct Comment(pub String);
