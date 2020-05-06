//!
//! The semantic analyzer type element.
//!

mod tests;

pub mod array;
pub mod contract;
pub mod enumeration;
pub mod error;
pub mod function;
pub mod range;
pub mod range_inclusive;
pub mod structure;
pub mod tuple;

use std::cell::RefCell;
use std::fmt;
use std::ops::Deref;
use std::rc::Rc;

use zinc_bytecode::builtins::BuiltinIdentifier;

use crate::lexical::token::location::Location;
use crate::semantic::analyzer::expression::error::Error as ExpressionError;
use crate::semantic::analyzer::expression::Analyzer as ExpressionAnalyzer;
use crate::semantic::analyzer::rule::Rule as TranslationRule;
use crate::semantic::element::constant::error::Error as ConstantError;
use crate::semantic::element::constant::Constant;
use crate::semantic::element::error::Error as ElementError;
use crate::semantic::element::r#type::error::Error as TypeError;
use crate::semantic::element::Element;
use crate::semantic::error::Error;
use crate::semantic::scope::item::Item as ScopeItem;
use crate::semantic::scope::Scope;
use crate::syntax::tree::r#type::variant::Variant as SyntaxTypeVariant;
use crate::syntax::tree::r#type::Type as SyntaxType;
use crate::syntax::tree::variant::Variant;

use self::array::Array;
use self::contract::Contract;
use self::enumeration::Enumeration;
use self::function::Function;
use self::range::Range;
use self::range_inclusive::RangeInclusive;
use self::structure::Structure;
use self::tuple::Tuple;

///
/// Describes a type.
///
#[derive(Debug, Clone)]
pub enum Type {
    /// the `()` type
    Unit(Option<Location>),
    /// the `bool` type
    Boolean(Option<Location>),
    /// the `u{N}` type
    IntegerUnsigned {
        location: Option<Location>,
        bitlength: usize,
    },
    /// the `i{N}` type
    IntegerSigned {
        location: Option<Location>,
        bitlength: usize,
    },
    /// the `field` type
    Field(Option<Location>),
    /// the compile-time only type used mostly for `dbg!` format strings and `assert!` messages
    String(Option<Location>),
    /// the compile-time only type used for loop bounds and array slicing
    Range(Range),
    /// the compile-time only type used for loop bounds and array slicing
    RangeInclusive(RangeInclusive),
    /// the ordinar array type
    Array(Array),
    /// the ordinar tuple type
    Tuple(Tuple),
    /// the ordinar structure type declared with a `struct` statement
    Structure(Structure),
    /// the ordinar enumeration type declared with an `enum` statement
    Enumeration(Enumeration),
    /// the special function type declared with an `fn` statement
    Function(Function),
    /// the special contract type declared with a `contract` statement
    Contract(Contract),
}

impl Type {
    pub fn unit(location: Option<Location>) -> Self {
        Self::Unit(location)
    }

    pub fn boolean(location: Option<Location>) -> Self {
        Self::Boolean(location)
    }

    pub fn integer_unsigned(location: Option<Location>, bitlength: usize) -> Self {
        Self::IntegerUnsigned {
            location,
            bitlength,
        }
    }

    pub fn integer_signed(location: Option<Location>, bitlength: usize) -> Self {
        Self::IntegerSigned {
            location,
            bitlength,
        }
    }

    pub fn integer(location: Option<Location>, is_signed: bool, bitlength: usize) -> Self {
        if is_signed {
            Self::integer_signed(location, bitlength)
        } else {
            Self::integer_unsigned(location, bitlength)
        }
    }

    pub fn field(location: Option<Location>) -> Self {
        Self::Field(location)
    }

    pub fn scalar(location: Option<Location>, is_signed: bool, bitlength: usize) -> Self {
        if is_signed {
            Self::integer_signed(location, bitlength)
        } else {
            match bitlength {
                crate::BITLENGTH_BOOLEAN => Self::boolean(location),
                crate::BITLENGTH_FIELD => Self::field(location),
                bitlength => Self::integer_unsigned(location, bitlength),
            }
        }
    }

    pub fn string(location: Option<Location>) -> Self {
        Self::String(location)
    }

    pub fn range(location: Option<Location>, r#type: Self) -> Self {
        Self::Range(Range::new(location, Box::new(r#type)))
    }

    pub fn range_inclusive(location: Option<Location>, r#type: Self) -> Self {
        Self::RangeInclusive(RangeInclusive::new(location, Box::new(r#type)))
    }

    pub fn array(location: Option<Location>, r#type: Self, size: usize) -> Self {
        Self::Array(Array::new(location, Box::new(r#type), size))
    }

    pub fn tuple(location: Option<Location>, types: Vec<Self>) -> Self {
        Self::Tuple(Tuple::new(location, types))
    }

    pub fn structure(
        location: Option<Location>,
        identifier: String,
        fields: Vec<(String, Self)>,
        scope: Option<Rc<RefCell<Scope>>>,
    ) -> Self {
        Self::Structure(Structure::new(location, identifier, fields, scope))
    }

    pub fn enumeration(
        location: Location,
        identifier: String,
        variants: Vec<Variant>,
        scope: Option<Rc<RefCell<Scope>>>,
    ) -> Result<Self, Error> {
        Enumeration::new(location, identifier, variants, scope).map(Self::Enumeration)
    }

    pub fn new_std_function(builtin_identifier: BuiltinIdentifier) -> Self {
        Self::Function(Function::new_std(builtin_identifier))
    }

    pub fn new_user_defined_function(
        location: Location,
        identifier: String,
        unique_id: usize,
        arguments: Vec<(String, Self)>,
        return_type: Self,
    ) -> Self {
        Self::Function(Function::new_user_defined(
            location,
            identifier,
            unique_id,
            arguments,
            return_type,
        ))
    }

    pub fn contract(
        location: Option<Location>,
        identifier: String,
        scope: Option<Rc<RefCell<Scope>>>,
    ) -> Self {
        Self::Contract(Contract::new(location, identifier, scope))
    }

    pub fn size(&self) -> usize {
        match self {
            Self::Unit(_) => 0,
            Self::Boolean(_) => 1,
            Self::IntegerUnsigned { .. } => 1,
            Self::IntegerSigned { .. } => 1,
            Self::Field(_) => 1,
            Self::String(_) => 0,
            Self::Range(_) => 0,
            Self::RangeInclusive(_) => 0,
            Self::Array(inner) => inner.r#type.size() * inner.size,
            Self::Tuple(inner) => inner.types.iter().map(|r#type| r#type.size()).sum(),
            Self::Structure(inner) => inner
                .fields
                .iter()
                .map(|(_name, r#type)| r#type.size())
                .sum(),
            Self::Enumeration(_inner) => 1,
            Self::Contract(_inner) => 0,
            Self::Function(_inner) => 0,
        }
    }

    pub fn is_scalar(&self) -> bool {
        match self {
            Self::Boolean(_) => true,
            Self::IntegerUnsigned { .. } => true,
            Self::IntegerSigned { .. } => true,
            Self::Field(_) => true,
            Self::Enumeration { .. } => true,
            _ => false,
        }
    }

    pub fn is_scalar_unsigned(&self) -> bool {
        match self {
            Self::IntegerUnsigned { .. } => true,
            Self::Field(_) => true,
            Self::Enumeration { .. } => true,
            _ => false,
        }
    }

    pub fn is_scalar_signed(&self) -> bool {
        match self {
            Self::IntegerSigned { .. } => true,
            _ => false,
        }
    }

    pub fn is_bit_array(&self) -> bool {
        match self {
            Self::Array(array) => array.r#type.deref() == &Self::boolean(None),
            _ => false,
        }
    }

    pub fn is_byte_array(&self) -> bool {
        match self {
            Self::Array(array) => {
                array.r#type.deref() == &Self::integer_unsigned(None, crate::BITLENGTH_BYTE)
            }
            _ => false,
        }
    }

    pub fn is_scalar_array(&self) -> bool {
        match self {
            Self::Array(array) => array.r#type.is_scalar(),
            _ => false,
        }
    }

    pub fn from_syntax_type(r#type: SyntaxType, scope: Rc<RefCell<Scope>>) -> Result<Self, Error> {
        let location = r#type.location;

        Ok(match r#type.variant {
            SyntaxTypeVariant::Unit => Self::unit(Some(location)),
            SyntaxTypeVariant::Boolean => Self::boolean(Some(location)),
            SyntaxTypeVariant::IntegerUnsigned { bitlength } => {
                Self::integer_unsigned(Some(location), bitlength)
            }
            SyntaxTypeVariant::IntegerSigned { bitlength } => {
                Self::integer_signed(Some(location), bitlength)
            }
            SyntaxTypeVariant::Field => Self::field(Some(location)),
            SyntaxTypeVariant::Array { inner, size } => {
                let r#type = Self::from_syntax_type(*inner, scope.clone())?;

                let size_location = size.location;
                let size = match ExpressionAnalyzer::new(scope, TranslationRule::Constant)
                    .analyze(size)?
                {
                    (Element::Constant(Constant::Integer(integer)), _intermediate) => {
                        integer.to_usize().map_err(|error| {
                            Error::Element(ElementError::Constant(ConstantError::Integer(error)))
                        })?
                    }
                    (element, _intermediate) => {
                        return Err(Error::Expression(ExpressionError::NonConstantElement {
                            location: size_location,
                            found: element.to_string(),
                        }));
                    }
                };

                Self::array(Some(location), r#type, size)
            }
            SyntaxTypeVariant::Tuple { inners } => {
                let mut types = Vec::with_capacity(inners.len());
                for inner in inners.into_iter() {
                    types.push(Self::from_syntax_type(inner, scope.clone())?);
                }
                Self::tuple(Some(location), types)
            }
            SyntaxTypeVariant::Alias { path } => {
                let location = path.location;
                match ExpressionAnalyzer::new(scope, TranslationRule::Type).analyze(path)? {
                    (Element::Type(r#type), _intermediate) => r#type,
                    (element, _intermediate) => {
                        return Err(Error::Element(ElementError::Type(
                            TypeError::AliasDoesNotPointToType {
                                location,
                                found: element.to_string(),
                            },
                        )));
                    }
                }
            }
        })
    }

    pub fn from_element(element: &Element, scope: Rc<RefCell<Scope>>) -> Result<Self, Error> {
        Ok(match element {
            Element::Value(value) => value.r#type(),
            Element::Constant(constant) => constant.r#type(),
            Element::Type(r#type) => r#type.to_owned(),
            Element::Path(path) => match Scope::resolve_path(scope, &path)? {
                ScopeItem::Variable(variable) => {
                    let mut r#type = variable.r#type;
                    r#type.set_location(path.last().location);
                    r#type
                }
                ScopeItem::Constant(constant) => {
                    let mut constant = constant.into_inner();
                    constant.set_location(path.last().location);
                    constant.r#type()
                }
                _ => panic!(crate::panic::VALIDATED_DURING_SYNTAX_ANALYSIS),
            },
            Element::Place(place) => {
                let mut r#type = place.r#type.to_owned();
                r#type.set_location(place.identifier.location);
                r#type
            }

            _ => panic!(crate::panic::VALIDATED_DURING_SYNTAX_ANALYSIS),
        })
    }

    pub fn set_location(&mut self, value: Location) {
        match self {
            Self::Unit(location) => *location = Some(value),
            Self::Boolean(location) => *location = Some(value),
            Self::IntegerUnsigned { location, .. } => *location = Some(value),
            Self::IntegerSigned { location, .. } => *location = Some(value),
            Self::Field(location) => *location = Some(value),
            Self::String(location) => *location = Some(value),
            Self::Range(inner) => inner.location = Some(value),
            Self::RangeInclusive(inner) => inner.location = Some(value),
            Self::Array(inner) => inner.location = Some(value),
            Self::Tuple(inner) => inner.location = Some(value),
            Self::Structure(inner) => inner.location = Some(value),
            Self::Enumeration(inner) => inner.location = Some(value),
            Self::Function(inner) => inner.set_location(value),
            Self::Contract(inner) => inner.location = Some(value),
        }
    }

    pub fn location(&self) -> Option<Location> {
        match self {
            Self::Unit(location) => *location,
            Self::Boolean(location) => *location,
            Self::IntegerUnsigned { location, .. } => *location,
            Self::IntegerSigned { location, .. } => *location,
            Self::Field(location) => *location,
            Self::String(location) => *location,
            Self::Range(inner) => inner.location,
            Self::RangeInclusive(inner) => inner.location,
            Self::Array(inner) => inner.location,
            Self::Tuple(inner) => inner.location,
            Self::Structure(inner) => inner.location,
            Self::Enumeration(inner) => inner.location,
            Self::Function(inner) => inner.location(),
            Self::Contract(inner) => inner.location,
        }
    }
}

impl PartialEq<Type> for Type {
    fn eq(&self, other: &Type) -> bool {
        match (self, other) {
            (Self::Unit(_), Self::Unit(_)) => true,
            (Self::Boolean(_), Self::Boolean(_)) => true,
            (
                Self::IntegerUnsigned { bitlength: b1, .. },
                Self::IntegerUnsigned { bitlength: b2, .. },
            ) => b1 == b2,
            (
                Self::IntegerSigned { bitlength: b1, .. },
                Self::IntegerSigned { bitlength: b2, .. },
            ) => b1 == b2,
            (Self::Field(_), Self::Field(_)) => true,
            (Self::String(_), Self::String(_)) => true,
            (Self::Range(inner_1), Self::Range(inner_2)) => inner_1.r#type == inner_2.r#type,
            (Self::RangeInclusive(inner_1), Self::RangeInclusive(inner_2)) => {
                inner_1.r#type == inner_2.r#type
            }
            (Self::Array(inner_1), Self::Array(inner_2)) => {
                inner_1.r#type == inner_2.r#type && inner_1.size == inner_2.size
            }
            (Self::Tuple(inner_1), Self::Tuple(inner_2)) => inner_1.types == inner_2.types,
            (Self::Structure(inner_1), Self::Structure(inner_2)) => inner_1 == inner_2,
            (Self::Enumeration(inner_1), Self::Enumeration(inner_2)) => inner_1 == inner_2,
            (Self::Contract(inner_1), Self::Contract(inner_2)) => inner_1 == inner_2,
            _ => false,
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Unit(_) => write!(f, "()"),
            Self::Boolean(_) => write!(f, "bool"),
            Self::IntegerUnsigned { bitlength, .. } => write!(f, "u{}", bitlength),
            Self::IntegerSigned { bitlength, .. } => write!(f, "i{}", bitlength),
            Self::Field(_) => write!(f, "field"),
            Self::String(_) => write!(f, "str"),
            Self::Range(inner) => write!(f, "{}", inner),
            Self::RangeInclusive(inner) => write!(f, "{}", inner),
            Self::Array(inner) => write!(f, "{}", inner),
            Self::Tuple(inner) => write!(f, "{}", inner),
            Self::Structure(inner) => write!(f, "{}", inner),
            Self::Enumeration(inner) => write!(f, "{}", inner),
            Self::Function(inner) => write!(f, "{}", inner),
            Self::Contract(inner) => write!(f, "{}", inner),
        }
    }
}
