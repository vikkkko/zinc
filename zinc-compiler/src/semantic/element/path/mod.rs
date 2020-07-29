//!
//! The semantic analyzer path element.
//!

use std::fmt;

use crate::lexical::token::location::Location;
use crate::syntax::tree::identifier::Identifier;

///
/// Paths are the `::` expressions which only exist at compile-time.
/// They are usually coerced to place, value, constant or type expressions.
///
#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    /// The location of the path expression.
    pub location: Location,
    /// The array of identifiers, which appear around the `::` operators.
    pub elements: Vec<Identifier>,
}

impl Path {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(location: Location, initial: Identifier) -> Self {
        Self {
            location,
            elements: vec![initial],
        }
    }

    ///
    /// Pushes another path identifier element.
    ///
    pub fn push_element(&mut self, element: Identifier) {
        self.elements.push(element);
    }

    ///
    /// The last path element, which is the actual name of the item accessed via the path.
    ///
    pub fn last(&self) -> &Identifier {
        self.elements
            .last()
            .expect(zinc_const::panic::VALIDATED_DURING_SYNTAX_ANALYSIS)
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            self.elements
                .iter()
                .map(|identifier| identifier.name.as_str())
                .collect::<Vec<&str>>()
                .join("::"),
        )
    }
}
