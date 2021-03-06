//!
//! The semantic analyzer scope.
//!

#[cfg(test)]
mod tests;

pub mod error;
pub mod intrinsic;
pub mod item;
pub mod memory_type;
pub mod stack;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::str;

use zinc_lexical::Keyword;
use zinc_lexical::Location;
use zinc_syntax::ConstStatement;
use zinc_syntax::ContractStatement;
use zinc_syntax::Identifier;

use crate::generator::statement::Statement as GeneratorStatement;
use crate::semantic::element::constant::Constant;
use crate::semantic::element::path::Path;
use crate::semantic::element::r#type::Type;
use crate::semantic::error::Error as SemanticError;
use crate::semantic::scope::intrinsic::IntrinsicTypeId;
use crate::source::Source;

use self::error::Error;
use self::intrinsic::IntrinsicScope;
use self::item::constant::Constant as ConstantItem;
use self::item::field::Field as FieldItem;
use self::item::module::Module as ModuleItem;
use self::item::r#type::statement::Statement as TypeStatementVariant;
use self::item::r#type::Type as TypeItem;
use self::item::variable::Variable as VariableItem;
use self::item::variant::Variant as VariantItem;
use self::item::Item;
use self::memory_type::MemoryType;

///
/// A scope consists of a hashmap of the declared items and a reference to its parent.
///
/// The global scope has the `root` scope as its parent with intrinsic functions and libraries.
///
/// Modules are connected to the entry scope hierarchy horizontally, being stored as module items.
///
#[derive(Debug, Clone)]
pub struct Scope {
    /// The scope name, that is, namespace name like module name, structure name, etc.
    name: String,
    /// The vertical parent scope, which the current one has access to.
    parent: Option<Rc<RefCell<Self>>>,
    /// The hashmap with items declared at the current scope level, with item names as keys.
    items: RefCell<HashMap<String, Rc<RefCell<Item>>>>,
    /// Whether the scope is the intrinsic one, that is, the root scope with intrinsic items.
    is_built_in: bool,
}

impl Scope {
    /// The scope items hashmap default capacity.
    const ITEMS_INITIAL_CAPACITY: usize = 1024;

    ///
    /// Initializes a scope with an explicit optional parent.
    ///
    /// Beware that if you omit the `parent`, intrinsic functions and `std` will not be available
    /// throughout the scope stack. To create a scope with such items available, use `new_global`.
    ///
    pub fn new(name: String, parent: Option<Rc<RefCell<Self>>>) -> Self {
        Self {
            name,
            parent,
            items: RefCell::new(HashMap::with_capacity(Self::ITEMS_INITIAL_CAPACITY)),
            is_built_in: false,
        }
    }

    ///
    /// Initializes a global scope without the intrinsic one as its parent.
    ///
    pub fn new_global(name: String) -> Self {
        Self {
            name,
            parent: Some(IntrinsicScope::initialize()),
            items: RefCell::new(HashMap::with_capacity(Self::ITEMS_INITIAL_CAPACITY)),
            is_built_in: false,
        }
    }

    ///
    /// Initializes the root scope with intrinsic function and library definitions.
    ///
    pub fn new_intrinsic(name: &'static str) -> Self {
        Self {
            name: name.to_owned(),
            parent: None,
            items: RefCell::new(HashMap::with_capacity(Self::ITEMS_INITIAL_CAPACITY)),
            is_built_in: true,
        }
    }

    ///
    /// Creates a child scope with `parent` as its parent.
    ///
    pub fn new_child(name: String, parent: Rc<RefCell<Scope>>) -> Rc<RefCell<Self>> {
        Self::new(name, Some(parent)).wrap()
    }

    ///
    /// Returns the scope parent.
    ///
    pub fn parent(&self) -> Option<Rc<RefCell<Self>>> {
        self.parent.to_owned()
    }

    ///
    /// Wraps the scope into `Rc<RefCell<_>>` simplifying most of initializations.
    ///
    pub fn wrap(self) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(self))
    }

    ///
    /// Internally defines all the items in the order they have been declared.
    ///
    pub fn define(&self) -> Result<(), SemanticError> {
        let mut items: Vec<(String, Rc<RefCell<Item>>)> =
            self.items.clone().into_inner().into_iter().collect();
        items.sort_by_key(|(_name, item)| item.borrow().item_id());

        for (name, item) in items.into_iter() {
            if Keyword::is_alias(name.as_str()) {
                continue;
            }

            item.borrow().define()?;
        }

        Ok(())
    }

    ///
    /// Inserts an item, does not check if the item has been already declared.
    ///
    pub fn insert_item(scope: Rc<RefCell<Scope>>, name: String, item: Rc<RefCell<Item>>) {
        scope.borrow().items.borrow_mut().insert(name, item);
    }

    ///
    /// Defines an item of arbitrary type, checks if the item has been already declared.
    ///
    pub fn define_item(
        scope: Rc<RefCell<Scope>>,
        identifier: Identifier,
        item: Rc<RefCell<Item>>,
    ) -> Result<(), SemanticError> {
        if let Ok(item) = scope.borrow().resolve_item(&identifier, true) {
            return Err(SemanticError::Scope(Error::ItemRedeclared {
                location: identifier.location,
                name: identifier.name.clone(),
                reference: item.borrow().location(),
            }));
        }

        scope
            .borrow()
            .items
            .borrow_mut()
            .insert(identifier.name, item);

        Ok(())
    }

    ///
    /// Defines a variable, which is usually a `let` binding or a function actual parameter.
    ///
    /// If the variable is the object instance `self` alias, it is not checked for being redeclared
    /// recursively to avoid collision with the module `self` alias.
    ///
    pub fn define_variable(
        scope: Rc<RefCell<Scope>>,
        identifier: Identifier,
        is_mutable: bool,
        r#type: Type,
        memory_type: MemoryType,
    ) -> Result<(), SemanticError> {
        if let Ok(item) = scope
            .borrow()
            .resolve_item(&identifier, !identifier.is_self_lowercase())
        {
            return Err(SemanticError::Scope(Error::ItemRedeclared {
                location: identifier.location,
                name: identifier.name.clone(),
                reference: item.borrow().location(),
            }));
        }

        let name = identifier.name.clone();
        let item = Item::Variable(VariableItem::new(
            Some(identifier.location),
            is_mutable,
            identifier.name,
            r#type,
            memory_type,
        ));

        scope.borrow().items.borrow_mut().insert(name, item.wrap());

        Ok(())
    }

    ///
    /// Defines a contract field.
    ///
    pub fn define_field(
        scope: Rc<RefCell<Scope>>,
        identifier: Identifier,
        r#type: Type,
        index: usize,
        is_public: bool,
        is_implicit: bool,
        is_immutable: bool,
    ) -> Result<(), SemanticError> {
        if let Ok(item) = scope.borrow().resolve_item(&identifier, false) {
            return Err(SemanticError::Scope(Error::ItemRedeclared {
                location: identifier.location,
                name: identifier.name.clone(),
                reference: item.borrow().location(),
            }));
        }

        let name = identifier.name.clone();
        let item = Item::Field(FieldItem::new(
            identifier.location,
            identifier.name,
            r#type,
            index,
            is_public,
            is_implicit,
            is_immutable,
        ));

        scope.borrow().items.borrow_mut().insert(name, item.wrap());

        Ok(())
    }

    ///
    /// Declares a constant, saving the `const` statement to define itself later during the second
    /// pass or referencing for the first time.
    ///
    pub fn declare_constant(
        scope: Rc<RefCell<Scope>>,
        statement: ConstStatement,
        is_associated: bool,
    ) -> Result<(), SemanticError> {
        if let Ok(item) = scope.borrow().resolve_item(&statement.identifier, true) {
            return Err(SemanticError::Scope(Error::ItemRedeclared {
                location: statement.location,
                name: statement.identifier.name.clone(),
                reference: item.borrow().location(),
            }));
        }

        let name = statement.identifier.name.clone();
        let item = Item::Constant(ConstantItem::new_declared(
            statement.identifier.location,
            statement,
            scope.clone(),
            is_associated,
        ));

        scope.borrow().items.borrow_mut().insert(name, item.wrap());

        Ok(())
    }

    ///
    /// Defines a constant, which has been instantly evaluated.
    ///
    pub fn define_constant(
        scope: Rc<RefCell<Scope>>,
        identifier: Identifier,
        constant: Constant,
        is_associated: bool,
    ) -> Result<(), SemanticError> {
        if let Ok(item) = scope.borrow().resolve_item(&identifier, true) {
            return Err(SemanticError::Scope(Error::ItemRedeclared {
                location: identifier.location,
                name: identifier.name.clone(),
                reference: item.borrow().location(),
            }));
        }

        let name = identifier.name;
        let item = Item::Constant(ConstantItem::new_defined(
            identifier.location,
            constant,
            is_associated,
        ));

        scope.borrow().items.borrow_mut().insert(name, item.wrap());

        Ok(())
    }

    ///
    /// Defines an enumeration variant, which has been instantly evaluated.
    ///
    pub fn define_variant(
        scope: Rc<RefCell<Scope>>,
        identifier: Identifier,
        constant: Constant,
    ) -> Result<(), SemanticError> {
        if let Ok(item) = scope.borrow().resolve_item(&identifier, false) {
            return Err(SemanticError::Scope(Error::ItemRedeclared {
                location: identifier.location,
                name: identifier.name.clone(),
                reference: item.borrow().location(),
            }));
        }

        let name = identifier.name;
        let item = Item::Variant(VariantItem::new(
            identifier.location,
            name.clone(),
            constant,
        ));

        scope.borrow().items.borrow_mut().insert(name, item.wrap());

        Ok(())
    }

    ///
    /// Declares a type, saving the `type`, `struct`, `enum`, `contract` or another statement to
    /// define itself later during the second pass or referencing for the first time.
    ///
    pub fn declare_type(
        scope: Rc<RefCell<Scope>>,
        statement: TypeStatementVariant,
        is_associated: bool,
    ) -> Result<(), SemanticError> {
        if let Ok(item) = scope.borrow().resolve_item(&statement.identifier(), true) {
            return Err(SemanticError::Scope(Error::ItemRedeclared {
                location: statement.location(),
                name: statement.identifier().name.to_owned(),
                reference: item.borrow().location(),
            }));
        }

        let name = statement.identifier().name.clone();
        let item = Item::Type(TypeItem::new_declared(
            Some(statement.location()),
            statement,
            scope.clone(),
            is_associated,
        )?);

        scope.borrow().items.borrow_mut().insert(name, item.wrap());

        Ok(())
    }

    ///
    /// Defines a type, which has been instantly evaluated.
    ///
    pub fn define_type(
        scope: Rc<RefCell<Scope>>,
        identifier: Identifier,
        r#type: Type,
        is_associated: bool,
        intermediate: Option<GeneratorStatement>,
    ) -> Result<(), SemanticError> {
        if let Ok(item) = scope.borrow().resolve_item(&identifier, true) {
            return Err(SemanticError::Scope(Error::ItemRedeclared {
                location: r#type.location().unwrap_or(identifier.location),
                name: identifier.name.clone(),
                reference: item.borrow().location(),
            }));
        }

        let name = identifier.name;
        let item = Item::Type(TypeItem::new_defined(
            Some(identifier.location),
            r#type,
            false,
            is_associated,
            intermediate,
        ));

        scope.borrow().items.borrow_mut().insert(name, item.wrap());

        Ok(())
    }

    ///
    /// Defines a `contract` type, also checks whether it is the only contract in the scope.
    ///
    pub fn declare_contract(
        scope: Rc<RefCell<Scope>>,
        statement: ContractStatement,
    ) -> Result<(), SemanticError> {
        if let Some(location) = scope.borrow().get_contract_location() {
            return Err(SemanticError::Scope(Error::ContractRedeclared {
                location: statement.location,
                reference: location,
            }));
        }

        Scope::declare_type(scope, TypeStatementVariant::Contract(statement), false)
    }

    ///
    /// Declares a module, saving its representation to define itself later during the second
    /// pass or referencing for the first time.
    ///
    pub fn declare_module(
        scope: Rc<RefCell<Scope>>,
        identifier: Identifier,
        module: Source,
        scope_crate: Rc<RefCell<Scope>>,
        is_entry: bool,
    ) -> Result<(), SemanticError> {
        if let Ok(item) = scope.borrow().resolve_item(&identifier, true) {
            return Err(SemanticError::Scope(Error::ItemRedeclared {
                location: identifier.location,
                name: identifier.name.clone(),
                reference: item.borrow().location(),
            }));
        }

        let name = identifier.name.clone();
        let module_scope = Self::new_global(identifier.name.clone()).wrap();
        let module = ModuleItem::new_declared(
            Some(identifier.location),
            module_scope.clone(),
            identifier.name,
            module,
            scope_crate,
            Some(scope.clone()),
            is_entry,
        )?;
        let item = Item::Module(module).wrap();

        module_scope
            .borrow()
            .items
            .borrow_mut()
            .insert(Keyword::SelfLowercase.to_string(), item.clone());
        scope.borrow().items.borrow_mut().insert(name, item);

        Ok(())
    }

    ///
    /// Returns the module `self` alias. Panics if the scope does not belong to a module or
    /// the alias has not been declared yet.
    ///
    pub fn get_module_self_alias(scope: Rc<RefCell<Scope>>) -> Rc<RefCell<Item>> {
        scope
            .borrow()
            .items
            .borrow()
            .get(&Keyword::SelfLowercase.to_string())
            .cloned()
            .expect(zinc_const::panic::VALIDATED_DURING_SEMANTIC_ANALYSIS)
    }

    ///
    /// Resolves an item at the specified path by looking through modules and type scopes.
    ///
    /// If the `path` consists of only one element, the path is resolved recursively, that is,
    /// looking through the whole scope hierarchy up to the module level and global intrinsic scope.
    ///
    /// If the `path` consists if more than one element, the elements starting from the 2nd are
    /// resolved non-recursively, that is, looking only at the first-level scope of the path element.
    ///
    /// Associated items are declared not in the namespace itself, but in the `Self` alias of it.
    /// So, if a path element's position is greater than 1 and the element is a namespace,
    /// the algorithm looks for the item in the `Self`-alias scope. It prevents the situation, when
    /// an item can be accessed from within other implementation items (e.g. methods) without
    /// specifying the `Self::` prefix.
    ///
    pub fn resolve_path(
        scope: Rc<RefCell<Scope>>,
        path: &Path,
    ) -> Result<Rc<RefCell<Item>>, SemanticError> {
        let mut current_scope = scope;

        for (index, identifier) in path.elements.iter().enumerate() {
            let is_element_first = index == 0;
            let is_element_last = index == path.elements.len() - 1;

            let item = current_scope
                .borrow()
                .resolve_item(identifier, is_element_first)?;
            item.borrow().define()?;

            if path.elements.len() == 1 && item.borrow().is_associated() {
                return Err(SemanticError::Scope(Error::AssociatedItemWithoutOwner {
                    location: path.location,
                    name: path.to_string(),
                }));
            }

            if is_element_last {
                return Ok(item);
            }

            current_scope = match *item.borrow() {
                Item::Module(ref module) => module.define()?,
                Item::Type(ref r#type) => {
                    let r#type = r#type.define()?;
                    match r#type {
                        Type::Enumeration(ref inner) => inner.scope.to_owned(),
                        Type::Structure(ref inner) => inner.scope.to_owned(),
                        Type::Contract(ref inner) => inner.scope.to_owned(),
                        _ => {
                            return Err(SemanticError::Scope(Error::ItemIsNotANamespace {
                                location: identifier.location,
                                name: identifier.name.to_owned(),
                            }))
                        }
                    }
                }
                _ => {
                    return Err(SemanticError::Scope(Error::ItemIsNotANamespace {
                        location: identifier.location,
                        name: identifier.name.to_owned(),
                    }))
                }
            };
        }

        Err(SemanticError::Scope(Error::ItemUndeclared {
            location: path.location,
            name: path.to_string(),
        }))
    }

    ///
    /// Resolves the item with `identifier` within the current `scope`. Looks through the parent scopes
    /// if `recursive` is true.
    ///
    pub fn resolve_item(
        &self,
        identifier: &Identifier,
        recursive: bool,
    ) -> Result<Rc<RefCell<Item>>, SemanticError> {
        match self.items.borrow().get(identifier.name.as_str()) {
            Some(item) => Ok(item.to_owned()),
            None => match self.parent {
                Some(ref parent) if recursive => {
                    parent.borrow().resolve_item(identifier, recursive)
                }
                Some(_) | None => Err(SemanticError::Scope(Error::ItemUndeclared {
                    location: identifier.location,
                    name: identifier.name.to_owned(),
                })),
            },
        }
    }

    ///
    /// Resolves the `std::collections::MTreeMap` type.
    ///
    /// An error is considered a bug, since the type is declared by the developer in the
    /// intrinsic module.
    ///
    pub fn resolve_mtreemap(location: Location, scope: Rc<RefCell<Scope>>) -> Type {
        match &*Scope::resolve_path(
            scope,
            &Path::new_complex(
                location,
                vec![
                    Identifier::new(location, "std".to_owned()),
                    Identifier::new(location, "collections".to_owned()),
                    Identifier::new(location, "MTreeMap".to_owned()),
                ],
            ),
        )
        .expect(zinc_const::panic::VALIDATED_DURING_SEMANTIC_ANALYSIS)
        .borrow()
        {
            Item::Type(r#type) => match r#type
                .define()
                .expect(zinc_const::panic::VALIDATED_DURING_SEMANTIC_ANALYSIS)
            {
                Type::Structure(mut structure)
                    if structure.type_id == IntrinsicTypeId::StdCollectionsMTreeMap as usize =>
                {
                    structure
                        .set_generics(
                            location,
                            Some(vec![
                                Type::integer_unsigned(None, zinc_const::bitlength::ETH_ADDRESS),
                                Type::integer_unsigned(None, zinc_const::bitlength::INTEGER_MAX),
                            ]),
                        )
                        .expect(zinc_const::panic::VALIDATED_DURING_SEMANTIC_ANALYSIS);
                    Type::Structure(structure)
                }
                _type => panic!(zinc_const::panic::VALIDATED_DURING_SEMANTIC_ANALYSIS),
            },
            _item => panic!(zinc_const::panic::VALIDATED_DURING_SEMANTIC_ANALYSIS),
        }
    }

    ///
    /// Gets the `main` function location from the current scope.
    ///
    pub fn get_main_location(&self) -> Option<Location> {
        self.items
            .borrow()
            .get(zinc_const::source::FUNCTION_MAIN_IDENTIFIER)
            .and_then(|main| main.borrow().location())
    }

    ///
    /// Gets the contract type definition from the current scope.
    ///
    pub fn get_contract_location(&self) -> Option<Location> {
        for (_name, item) in self.items.borrow().iter() {
            match *item.borrow() {
                Item::Type(ref r#type) if r#type.is_contract() => return item.borrow().location(),
                _ => {}
            }
        }

        None
    }

    ///
    /// Extracts the intermediate representation from the element.
    ///
    pub fn get_intermediate(&self) -> Vec<GeneratorStatement> {
        self.items
            .borrow()
            .iter()
            .filter_map(|(name, item)| {
                if Keyword::is_alias(name.as_str()) {
                    return None;
                }

                Some(item.borrow().get_intermediate())
            })
            .flatten()
            .collect()
    }

    ///
    /// Returns the scope name.
    ///
    pub fn name(&self) -> String {
        self.name.to_owned()
    }

    ///
    /// Displays the scope hierarchy.
    ///
    /// Is used for testing purposes.
    ///
    pub fn show(&self, level: usize) {
        println!("{}==== Scope <{}> ====", "    ".repeat(level), self.name);

        for (name, item) in self.items.borrow().iter() {
            println!("{}{}: {}", "    ".repeat(level), name, item.borrow());

            if Keyword::is_alias(name.as_str()) {
                continue;
            }

            if let Item::Module(ref module) = *item.borrow() {
                match module.scope() {
                    Ok(scope) => scope.borrow().show(level + 1),
                    Err(error) => log::warn!("SCOPE IS UNAVAILABLE: {:?}", error),
                }
            }
        }

        if let Some(parent) = self.parent.as_ref() {
            if parent.borrow().is_built_in {
                return;
            }

            parent.borrow().show(level + 1);
        }
    }
}
