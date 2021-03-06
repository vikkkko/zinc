//!
//! The semantic analyzer structure type element.
//!

#[cfg(test)]
mod tests;

pub mod error;

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use zinc_lexical::Location;

use crate::semantic::element::r#type::Type;
use crate::semantic::scope::Scope;

use self::error::Error;

///
/// Describes a structure type.
///
/// Consists of the local structure `identifier` within its scope, global `type_id`, `fields`,
/// and the implementation `scope`, which contains the reference to its parent scope.
///
#[derive(Debug, Clone)]
pub struct Structure {
    /// The structure type location in the code.
    pub location: Option<Location>,
    /// The structure type identifier.
    pub identifier: String,
    /// The unique structure type ID.
    pub type_id: usize,
    /// The ordered list of the structure fields.
    pub fields: Vec<(String, Type)>,
    /// The ordered list of the structure generic type formal arguments.
    pub generics: Option<Vec<String>>,
    /// The structure generic type actual arguments.
    /// These are set upon the structure value initialization, where arguments are set in `<...>`.
    pub params: Option<HashMap<String, Type>>,
    /// The structure scope, where its methods and associated items are declared.
    pub scope: Rc<RefCell<Scope>>,
}

impl Structure {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(
        location: Option<Location>,
        identifier: String,
        type_id: usize,
        fields: Vec<(String, Type)>,
        generics: Option<Vec<String>>,
        params: Option<HashMap<String, Type>>,
        scope: Option<Rc<RefCell<Scope>>>,
    ) -> Self {
        let scope = scope.unwrap_or_else(|| Scope::new(identifier.clone(), None).wrap());

        Self {
            location,
            identifier,
            type_id,
            fields,
            generics,
            params,
            scope,
        }
    }

    ///
    /// Validates and sets the generic type arguments.
    ///
    pub fn set_generics(
        &mut self,
        location: Location,
        generics: Option<Vec<Type>>,
    ) -> Result<(), Error> {
        match (self.generics.as_ref(), generics) {
            (Some(formal), Some(actual)) => {
                if formal.len() != actual.len() {
                    return Err(Error::InvalidGenericsNumber {
                        location,
                        type_identifier: self.identifier.to_owned(),
                        expected: formal.len(),
                        found: actual.len(),
                    });
                }

                let mut params = HashMap::with_capacity(actual.len());
                for (name, r#type) in formal.iter().zip(actual.into_iter()) {
                    params.insert(name.to_owned(), r#type);
                }
                self.params = Some(params);

                Ok(())
            }
            (Some(names), None) => Err(Error::ExpectedGenerics {
                location,
                type_identifier: self.identifier.to_owned(),
                expected: names.len(),
            }),
            (None, Some(_types)) => Err(Error::UnexpectedGenerics {
                location,
                type_identifier: self.identifier.to_owned(),
            }),
            (None, None) => Ok(()),
        }
    }
}

impl PartialEq<Self> for Structure {
    fn eq(&self, other: &Self) -> bool {
        self.type_id == other.type_id
    }
}

impl fmt::Display for Structure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.identifier)
    }
}
