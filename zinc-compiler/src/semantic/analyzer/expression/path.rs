//!
//! The path expression translator.
//!

use std::cell::RefCell;
use std::rc::Rc;

use crate::generator::expression::operand::constant::Constant as GeneratorConstant;
use crate::generator::expression::operand::Operand as GeneratorExpressionOperand;
use crate::generator::r#type::Type as GeneratorType;
use crate::semantic::analyzer::expression::error::Error as ExpressionError;
use crate::semantic::analyzer::rule::Rule as TranslationRule;
use crate::semantic::element::error::Error as ElementError;
use crate::semantic::element::path::Path;
use crate::semantic::element::place::Place;
use crate::semantic::element::r#type::i_typed::ITyped;
use crate::semantic::element::r#type::Type;
use crate::semantic::element::value::error::Error as ValueError;
use crate::semantic::element::value::structure::error::Error as StructureValueError;
use crate::semantic::element::value::Value;
use crate::semantic::element::Element;
use crate::semantic::error::Error;
use crate::semantic::scope::item::Item as ScopeItem;
use crate::semantic::scope::memory_type::MemoryType;
use crate::semantic::scope::Scope;

///
/// The path expression translator.
///
pub struct Translator {}

impl Translator {
    ///
    /// Translates the path expression to a semantic expression type specified in `rule`.
    ///
    pub fn translate(
        scope: Rc<RefCell<Scope>>,
        path: Path,
        rule: TranslationRule,
    ) -> Result<(Element, Option<GeneratorExpressionOperand>), Error> {
        let location = path.location;
        let path_last_identifier = path.last().to_owned();

        match rule {
            TranslationRule::Place => match *Scope::resolve_path(scope, &path)?.borrow() {
                ScopeItem::Variable(ref variable) => Ok((
                    Element::Place(Place::new(
                        path_last_identifier,
                        variable.r#type.to_owned(),
                        variable.is_mutable,
                        variable.memory_type.to_owned().into(),
                    )),
                    None,
                )),
                ScopeItem::Field(ref field) => Ok((
                    Element::Place(Place::new(
                        path_last_identifier,
                        field.r#type.to_owned(),
                        false,
                        MemoryType::ContractStorage { index: field.index }.into(),
                    )),
                    None,
                )),
                ScopeItem::Constant(ref constant) => {
                    let mut constant = constant.define()?;
                    constant.set_location(location);

                    let intermediate = GeneratorConstant::try_from_semantic(&constant);

                    Ok((
                        Element::Constant(constant),
                        intermediate.map(GeneratorExpressionOperand::Constant),
                    ))
                }
                ScopeItem::Variant(ref variant) => {
                    let mut constant = variant.constant.to_owned();
                    constant.set_location(location);

                    let intermediate = GeneratorConstant::try_from_semantic(&constant);

                    Ok((
                        Element::Constant(constant),
                        intermediate.map(GeneratorExpressionOperand::Constant),
                    ))
                }
                ScopeItem::Type(ref r#type) => {
                    let mut r#type = r#type.define()?;
                    r#type.set_location(location);

                    Ok((Element::Type(r#type), None))
                }
                ScopeItem::Module(_) => Ok((Element::Module(path_last_identifier), None)),
            },
            TranslationRule::Value => match *Scope::resolve_path(scope, &path)?.borrow() {
                ScopeItem::Variable(ref variable) => {
                    let value = Value::try_from_type(&variable.r#type, false, Some(location))
                        .map_err(ElementError::Value)
                        .map_err(Error::Element)?;
                    let r#type = value.r#type();
                    let element = Element::Value(value);

                    let intermediate = GeneratorType::try_from_semantic(&r#type)
                        .map(|_| {
                            Place::new(
                                path_last_identifier,
                                r#type,
                                variable.is_mutable,
                                variable.memory_type.to_owned().into(),
                            )
                            .into()
                        })
                        .map(GeneratorExpressionOperand::Place);

                    Ok((element, intermediate))
                }
                ScopeItem::Field(ref field) => {
                    let value = Value::try_from_type(&field.r#type, false, Some(location))
                        .map_err(ElementError::Value)
                        .map_err(Error::Element)?;
                    let r#type = value.r#type();
                    let element = Element::Value(value);

                    let intermediate = GeneratorType::try_from_semantic(&r#type)
                        .map(|_| {
                            Place::new(
                                path_last_identifier,
                                r#type,
                                false,
                                MemoryType::ContractStorage { index: field.index }.into(),
                            )
                            .into()
                        })
                        .map(GeneratorExpressionOperand::Place);

                    Ok((element, intermediate))
                }
                ScopeItem::Constant(ref constant) => {
                    let mut constant = constant.define()?;
                    constant.set_location(location);

                    let intermediate = GeneratorConstant::try_from_semantic(&constant)
                        .map(GeneratorExpressionOperand::Constant);

                    let element = Element::Constant(constant);
                    Ok((element, intermediate))
                }
                ScopeItem::Variant(ref variant) => {
                    let mut constant = variant.constant.to_owned();
                    constant.set_location(location);

                    let intermediate = GeneratorConstant::try_from_semantic(&constant)
                        .map(GeneratorExpressionOperand::Constant);

                    let element = Element::Constant(constant);
                    Ok((element, intermediate))
                }
                ScopeItem::Type(ref r#type) => {
                    let mut r#type = r#type.define()?;
                    r#type.set_location(location);

                    match r#type {
                        Type::Structure(structure) if !structure.fields.is_empty() => {
                            return Err(Error::Element(ElementError::Value(ValueError::Structure(
                                StructureValueError::NotInitialized {
                                    location,
                                    type_identifier: structure.identifier,
                                },
                            ))))
                        }
                        _ => {}
                    }

                    Ok((Element::Type(r#type), None))
                }
                ScopeItem::Module(_) => Ok((Element::Module(path_last_identifier), None)),
            },
            TranslationRule::Constant => match *Scope::resolve_path(scope, &path)?.borrow() {
                ScopeItem::Constant(ref constant) => {
                    let mut constant = constant.define()?;
                    constant.set_location(location);

                    let intermediate = GeneratorConstant::try_from_semantic(&constant)
                        .map(GeneratorExpressionOperand::Constant);

                    let element = Element::Constant(constant);
                    Ok((element, intermediate))
                }
                ScopeItem::Variant(ref variant) => {
                    let mut constant = variant.constant.to_owned();
                    constant.set_location(location);

                    let intermediate = GeneratorConstant::try_from_semantic(&constant);

                    Ok((
                        Element::Constant(constant),
                        intermediate.map(GeneratorExpressionOperand::Constant),
                    ))
                }
                ref item => Err(Error::Expression(ExpressionError::NonConstantElement {
                    location: path.location,
                    found: item.to_string(),
                })),
            },

            TranslationRule::Type => match *Scope::resolve_path(scope, &path)?.borrow() {
                ScopeItem::Type(ref r#type) => {
                    let mut r#type = r#type.define()?;
                    if !r#type.is_source_function() {
                        r#type.set_location(location);
                    }

                    Ok((Element::Type(r#type), None))
                }
                _ => Ok((Element::Path(path), None)),
            },
            TranslationRule::Path => Ok((Element::Path(path), None)),
            TranslationRule::Field => Ok((Element::Identifier(path_last_identifier), None)),
        }
    }
}
