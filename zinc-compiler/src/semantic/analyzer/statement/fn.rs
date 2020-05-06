//!
//! The `fn` statement semantic analyzer.
//!

use std::cell::RefCell;
use std::rc::Rc;

use crate::generator::statement::function::Statement as GeneratorFunctionStatement;
use crate::lexical::token::lexeme::keyword::Keyword;
use crate::semantic::analyzer::expression::block::Analyzer as BlockAnalyzer;
use crate::semantic::analyzer::rule::Rule as TranslationRule;
use crate::semantic::element::error::Error as ElementError;
use crate::semantic::element::r#type::error::Error as TypeError;
use crate::semantic::element::r#type::function::error::Error as FunctionTypeError;
use crate::semantic::element::r#type::function::user::Function as UserDefinedFunctionType;
use crate::semantic::element::r#type::function::Function as FunctionType;
use crate::semantic::element::r#type::Type;
use crate::semantic::error::Error;
use crate::semantic::scope::item::r#type::index::SOFT as TYPE_INDEX_SOFT;
use crate::semantic::scope::item::r#type::Type as ScopeTypeItem;
use crate::semantic::scope::item::variable::Variable as ScopeVariableItem;
use crate::semantic::scope::stack::Stack as ScopeStack;
use crate::semantic::scope::Scope;
use crate::syntax::tree::identifier::Identifier;
use crate::syntax::tree::pattern_binding::variant::Variant as BindingPatternVariant;
use crate::syntax::tree::statement::r#fn::Statement as FnStatement;

///
/// The context lets the analyzer know of file type where the analyzed statements are defined.
///
pub enum Context {
    /// The module root namespace.
    Module,
    /// The ordinar implementation namespace.
    Implementation,
    /// The contract definition namespace.
    Contract,
}

pub struct Analyzer {}

impl Analyzer {
    ///
    /// Analyzes the function statement.
    ///
    pub fn analyze(
        scope: Rc<RefCell<Scope>>,
        statement: FnStatement,
        context: Context,
    ) -> Result<Option<GeneratorFunctionStatement>, Error> {
        if let Context::Contract = context {
            if statement.is_public && statement.is_constant {
                return Err(Error::EntryPointConstant {
                    location: statement.location,
                });
            }
        }

        if statement.is_constant {
            Err(Error::ForbiddenConstantFunction {
                location: statement.location,
            })
        } else {
            Self::runtime(scope, statement, context).map(Option::Some)
        }
    }

    ///
    /// Analyzes a runtime statement and returns its IR for the next compiler phase.
    ///
    fn runtime(
        scope: Rc<RefCell<Scope>>,
        statement: FnStatement,
        context: Context,
    ) -> Result<GeneratorFunctionStatement, Error> {
        let location = statement.location;

        let mut scope_stack = ScopeStack::new(scope);

        let mut arguments = Vec::with_capacity(statement.argument_bindings.len());
        for (index, argument_binding) in statement.argument_bindings.iter().enumerate() {
            let identifier = match argument_binding.variant {
                BindingPatternVariant::Binding { ref identifier, .. } => identifier.name.to_owned(),
                BindingPatternVariant::Wildcard => continue,
                BindingPatternVariant::SelfAlias { .. } => {
                    if index != 0 {
                        return Err(Error::Element(ElementError::Type(TypeError::Function(
                            FunctionTypeError::FunctionMethodSelfNotFirst {
                                location: statement.identifier.location,
                                function: statement.identifier.name.clone(),
                                position: index + 1,
                                reference: argument_binding.location,
                            },
                        ))));
                    }

                    Keyword::SelfLowercase.to_string()
                }
            };

            arguments.push((
                identifier,
                Type::from_syntax_type(argument_binding.r#type.to_owned(), scope_stack.top())?,
            ));
        }

        let expected_type = match statement.return_type {
            Some(ref r#type) => Type::from_syntax_type(r#type.to_owned(), scope_stack.top())?,
            None => Type::unit(None),
        };

        let unique_id = TYPE_INDEX_SOFT.next(statement.identifier.name.clone());
        let function_type = UserDefinedFunctionType::new(
            statement.location,
            statement.identifier.name.clone(),
            unique_id,
            arguments.clone(),
            expected_type.clone(),
        );
        let r#type = Type::Function(FunctionType::UserDefined(function_type));

        Scope::declare_type(
            scope_stack.top(),
            statement.identifier.clone(),
            ScopeTypeItem::new(Some(location), r#type),
        )?;

        scope_stack.push();
        for argument_binding in statement.argument_bindings.into_iter() {
            match argument_binding.variant {
                BindingPatternVariant::Binding {
                    identifier,
                    is_mutable,
                } => {
                    let location = identifier.location;

                    let r#type =
                        Type::from_syntax_type(argument_binding.r#type, scope_stack.top())?;

                    Scope::declare_variable(
                        scope_stack.top(),
                        identifier,
                        ScopeVariableItem::new(location, is_mutable, r#type),
                    )?;
                }
                BindingPatternVariant::Wildcard => continue,
                BindingPatternVariant::SelfAlias {
                    location,
                    is_mutable,
                } => {
                    let identifier = Identifier::new(location, Keyword::SelfLowercase.to_string());
                    let r#type =
                        Type::from_syntax_type(argument_binding.r#type, scope_stack.top())?;

                    Scope::declare_variable(
                        scope_stack.top(),
                        identifier,
                        ScopeVariableItem::new(location, is_mutable, r#type),
                    )?;
                }
            }
        }

        let return_expression_location = match statement
            .body
            .expression
            .as_ref()
            .map(|expression| expression.location)
        {
            Some(location) => location,
            None => statement
                .body
                .statements
                .last()
                .map(|statement| statement.location())
                .unwrap_or(statement.location),
        };

        let rule = if statement.is_constant {
            TranslationRule::Constant
        } else {
            TranslationRule::Value
        };
        let (result, body) = BlockAnalyzer::analyze(scope_stack.top(), statement.body, rule)?;
        scope_stack.pop();

        let result_type = Type::from_element(&result, scope_stack.top())?;
        if expected_type != result_type {
            return Err(Error::Element(ElementError::Type(TypeError::Function(
                FunctionTypeError::ReturnType {
                    location: return_expression_location,
                    function: statement.identifier.name.clone(),
                    expected: expected_type.to_string(),
                    found: result_type.to_string(),
                    reference: statement
                        .return_type
                        .map(|r#type| r#type.location)
                        .unwrap_or(statement.location),
                },
            ))));
        }

        let is_main = statement.identifier.name.as_str() == crate::FUNCTION_MAIN_IDENTIFIER;

        let is_contract_entry = if let Context::Contract = context {
            statement.is_public
        } else {
            false
        };

        Ok(GeneratorFunctionStatement::new(
            location,
            statement.identifier.name,
            arguments,
            body,
            expected_type,
            unique_id,
            is_contract_entry,
            is_main,
        ))
    }
}
