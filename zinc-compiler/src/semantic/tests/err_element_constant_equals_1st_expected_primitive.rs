//!
//! A semantic analyzer test.
//!

#![cfg(test)]

use crate::lexical::Location;

use crate::semantic::Constant;
use crate::semantic::ConstantError;
use crate::semantic::ElementError;
use crate::semantic::Error as SemanticError;

use crate::Error;

#[test]
fn test() {
    let input = r#"
fn main() {
    let value = "string" == 42;
}
"#;

    let expected = Err(Error::Semantic(SemanticError::Element(
        Location::new(3, 26),
        ElementError::Constant(
            ConstantError::OperatorEqualsFirstOperandExpectedPrimitiveType(Constant::String(
                "string".to_owned(),
            )),
        ),
    )));

    let result = super::get_binary_result(input);

    assert_eq!(expected, result);
}
