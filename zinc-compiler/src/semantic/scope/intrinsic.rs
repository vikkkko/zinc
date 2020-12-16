//!
//! The semantic analyzer scope intrinsic items.
//!

use std::cell::RefCell;
use std::rc::Rc;

use zinc_build::LibraryFunctionIdentifier;

use crate::semantic::element::r#type::function::Function as FunctionType;
use crate::semantic::element::r#type::structure::Structure as StructureType;
use crate::semantic::element::r#type::Type;
use crate::semantic::scope::item::module::Module as ScopeModuleItem;
use crate::semantic::scope::item::r#type::Type as ScopeTypeItem;
use crate::semantic::scope::item::variable::Variable as ScopeVariableItem;
use crate::semantic::scope::item::Item as ScopeItem;
use crate::semantic::scope::memory_type::MemoryType;
use crate::semantic::scope::Scope;

///
/// An intrinsic items set instance creator.
///
/// The intrinsic items are functions `dbg!` and `require` and the `std` and `zksync` libraries.
///
#[derive(Debug)]
pub struct IntrinsicScope {}

///
/// The intrinsic structures type IDs.
///
pub enum IntrinsicTypeId {
    /// The `std::crypto::ecc::Point` structure type ID.
    StdCryptoEccPoint = 0,
    /// The `std::crypto::schnorr::Signature` structure type ID.
    StdCryptoSchnorrSignature = 1,
    /// The `zksync::Transaction` structure type ID.
    ZkSyncTransaction = 2,
    /// The `std::collections::MTreeMap` structure type ID.
    StdCollectionsMTreeMap = 3,
}

impl IntrinsicScope {
    ///
    /// Initializes the intrinsic module scope.
    ///
    pub fn initialize() -> Rc<RefCell<Scope>> {
        let scope = Scope::new_intrinsic("intrinsic").wrap();

        let function_dbg = FunctionType::new_dbg();
        Scope::insert_item(
            scope.clone(),
            function_dbg.identifier(),
            ScopeItem::Type(ScopeTypeItem::new_built_in(
                Type::Function(function_dbg),
                false,
            ))
            .wrap(),
        );

        let function_require = FunctionType::new_require();
        Scope::insert_item(
            scope.clone(),
            function_require.identifier(),
            ScopeItem::Type(ScopeTypeItem::new_built_in(
                Type::Function(function_require),
                false,
            ))
            .wrap(),
        );

        Scope::insert_item(
            scope.clone(),
            "std".to_owned(),
            ScopeItem::Module(ScopeModuleItem::new_built_in(
                "std".to_owned(),
                Self::module_std(),
            ))
            .wrap(),
        );

        Scope::insert_item(
            scope.clone(),
            "zksync".to_owned(),
            ScopeItem::Module(ScopeModuleItem::new_built_in(
                "zksync".to_owned(),
                Self::module_zksync(),
            ))
            .wrap(),
        );

        scope
    }

    ///
    /// Initializes the `std` module scope.
    ///
    fn module_std() -> Rc<RefCell<Scope>> {
        let scope = Scope::new_intrinsic("std").wrap();

        Scope::insert_item(
            scope.clone(),
            "crypto".to_owned(),
            ScopeItem::Module(ScopeModuleItem::new_built_in(
                "crypto".to_owned(),
                Self::module_crypto(),
            ))
            .wrap(),
        );
        Scope::insert_item(
            scope.clone(),
            "convert".to_owned(),
            ScopeItem::Module(ScopeModuleItem::new_built_in(
                "convert".to_owned(),
                Self::module_convert(),
            ))
            .wrap(),
        );
        Scope::insert_item(
            scope.clone(),
            "array".to_owned(),
            ScopeItem::Module(ScopeModuleItem::new_built_in(
                "array".to_owned(),
                Self::module_array(),
            ))
            .wrap(),
        );
        Scope::insert_item(
            scope.clone(),
            "ff".to_owned(),
            ScopeItem::Module(ScopeModuleItem::new_built_in(
                "ff".to_owned(),
                Self::module_ff(),
            ))
            .wrap(),
        );
        Scope::insert_item(
            scope.clone(),
            "collections".to_owned(),
            ScopeItem::Module(ScopeModuleItem::new_built_in(
                "collections".to_owned(),
                Self::module_collections(),
            ))
            .wrap(),
        );

        scope
    }

    ///
    /// Initializes the `std::crypto` module scope.
    ///
    fn module_crypto() -> Rc<RefCell<Scope>> {
        let scope = Scope::new_intrinsic("crypto").wrap();

        let sha256 = FunctionType::new_library(LibraryFunctionIdentifier::CryptoSha256);
        let pedersen = FunctionType::new_library(LibraryFunctionIdentifier::CryptoPedersen);

        let schnorr_scope = Scope::new_intrinsic("schnorr").wrap();
        let schnorr_signature_scope = Scope::new_intrinsic("Signature").wrap();
        let schnorr_verify =
            FunctionType::new_library(LibraryFunctionIdentifier::CryptoSchnorrSignatureVerify);
        Scope::insert_item(
            schnorr_signature_scope.clone(),
            schnorr_verify.identifier(),
            ScopeItem::Type(ScopeTypeItem::new_built_in(
                Type::Function(schnorr_verify),
                true,
            ))
            .wrap(),
        );
        let ecc_point = StructureType::new(
            None,
            "Point".to_owned(),
            IntrinsicTypeId::StdCryptoEccPoint as usize,
            vec![
                ("x".to_owned(), Type::field(None)),
                ("y".to_owned(), Type::field(None)),
            ],
            None,
            None,
            None,
        );
        let schnorr_signature = StructureType::new(
            None,
            schnorr_signature_scope.borrow().name(),
            IntrinsicTypeId::StdCryptoSchnorrSignature as usize,
            vec![
                ("r".to_owned(), Type::Structure(ecc_point.clone())),
                ("s".to_owned(), Type::field(None)),
                ("pk".to_owned(), Type::Structure(ecc_point.clone())),
            ],
            None,
            None,
            Some(schnorr_signature_scope.clone()),
        );
        Scope::insert_item(
            schnorr_scope.clone(),
            schnorr_signature_scope.borrow().name(),
            ScopeItem::Type(ScopeTypeItem::new_built_in(
                Type::Structure(schnorr_signature),
                false,
            ))
            .wrap(),
        );

        let ecc_scope = Scope::new_intrinsic("ecc").wrap();
        Scope::insert_item(
            ecc_scope.clone(),
            ecc_point.identifier.clone(),
            ScopeItem::Type(ScopeTypeItem::new_built_in(
                Type::Structure(ecc_point),
                false,
            ))
            .wrap(),
        );

        Scope::insert_item(
            scope.clone(),
            sha256.identifier(),
            ScopeItem::Type(ScopeTypeItem::new_built_in(Type::Function(sha256), false)).wrap(),
        );
        Scope::insert_item(
            scope.clone(),
            pedersen.identifier(),
            ScopeItem::Type(ScopeTypeItem::new_built_in(Type::Function(pedersen), false)).wrap(),
        );
        Scope::insert_item(
            scope.clone(),
            ecc_scope.borrow().name(),
            ScopeItem::Module(ScopeModuleItem::new_built_in(
                ecc_scope.borrow().name(),
                ecc_scope.clone(),
            ))
            .wrap(),
        );
        Scope::insert_item(
            scope.clone(),
            schnorr_scope.borrow().name(),
            ScopeItem::Module(ScopeModuleItem::new_built_in(
                schnorr_scope.borrow().name(),
                schnorr_scope.clone(),
            ))
            .wrap(),
        );

        scope
    }

    ///
    /// Initializes the `std::convert` module scope.
    ///
    fn module_convert() -> Rc<RefCell<Scope>> {
        let scope = Scope::new_intrinsic("convert").wrap();

        let to_bits = FunctionType::new_library(LibraryFunctionIdentifier::ConvertToBits);
        let from_bits_unsigned =
            FunctionType::new_library(LibraryFunctionIdentifier::ConvertFromBitsUnsigned);
        let from_bits_signed =
            FunctionType::new_library(LibraryFunctionIdentifier::ConvertFromBitsSigned);
        let from_bits_field =
            FunctionType::new_library(LibraryFunctionIdentifier::ConvertFromBitsField);

        Scope::insert_item(
            scope.clone(),
            to_bits.identifier(),
            ScopeItem::Type(ScopeTypeItem::new_built_in(Type::Function(to_bits), false)).wrap(),
        );
        Scope::insert_item(
            scope.clone(),
            from_bits_unsigned.identifier(),
            ScopeItem::Type(ScopeTypeItem::new_built_in(
                Type::Function(from_bits_unsigned),
                false,
            ))
            .wrap(),
        );
        Scope::insert_item(
            scope.clone(),
            from_bits_signed.identifier(),
            ScopeItem::Type(ScopeTypeItem::new_built_in(
                Type::Function(from_bits_signed),
                false,
            ))
            .wrap(),
        );
        Scope::insert_item(
            scope.clone(),
            from_bits_field.identifier(),
            ScopeItem::Type(ScopeTypeItem::new_built_in(
                Type::Function(from_bits_field),
                false,
            ))
            .wrap(),
        );

        scope
    }

    ///
    /// Initializes the `std::array` module scope.
    ///
    fn module_array() -> Rc<RefCell<Scope>> {
        let scope = Scope::new_intrinsic("array").wrap();

        let reverse = FunctionType::new_library(LibraryFunctionIdentifier::ArrayReverse);
        let truncate = FunctionType::new_library(LibraryFunctionIdentifier::ArrayTruncate);
        let pad = FunctionType::new_library(LibraryFunctionIdentifier::ArrayPad);

        Scope::insert_item(
            scope.clone(),
            reverse.identifier(),
            ScopeItem::Type(ScopeTypeItem::new_built_in(Type::Function(reverse), false)).wrap(),
        );
        Scope::insert_item(
            scope.clone(),
            truncate.identifier(),
            ScopeItem::Type(ScopeTypeItem::new_built_in(Type::Function(truncate), false)).wrap(),
        );
        Scope::insert_item(
            scope.clone(),
            pad.identifier(),
            ScopeItem::Type(ScopeTypeItem::new_built_in(Type::Function(pad), false)).wrap(),
        );

        scope
    }

    ///
    /// Initializes the `std::ff` module scope.
    ///
    fn module_ff() -> Rc<RefCell<Scope>> {
        let scope = Scope::new_intrinsic("ff").wrap();

        let invert = FunctionType::new_library(LibraryFunctionIdentifier::FfInvert);

        Scope::insert_item(
            scope.clone(),
            invert.identifier(),
            ScopeItem::Type(ScopeTypeItem::new_built_in(Type::Function(invert), false)).wrap(),
        );

        scope
    }

    ///
    /// Initializes the `std::collections` module scope.
    ///
    fn module_collections() -> Rc<RefCell<Scope>> {
        let scope = Scope::new_intrinsic("collections").wrap();

        let merkle_tree_map_scope = Scope::new_intrinsic("MTreeMap").wrap();
        let merkle_tree_map = StructureType::new(
            None,
            "MTreeMap".to_owned(),
            IntrinsicTypeId::StdCollectionsMTreeMap as usize,
            vec![],
            Some(vec!["K".to_owned(), "V".to_owned()]),
            None,
            Some(merkle_tree_map_scope.clone()),
        );
        let merkle_tree_map_get =
            FunctionType::new_library(LibraryFunctionIdentifier::CollectionsMTreeMapGet);
        Scope::insert_item(
            merkle_tree_map_scope.clone(),
            merkle_tree_map_get.identifier(),
            ScopeItem::Type(ScopeTypeItem::new_built_in(
                Type::Function(merkle_tree_map_get),
                true,
            ))
            .wrap(),
        );
        let merkle_tree_map_contains =
            FunctionType::new_library(LibraryFunctionIdentifier::CollectionsMTreeMapContains);
        Scope::insert_item(
            merkle_tree_map_scope.clone(),
            merkle_tree_map_contains.identifier(),
            ScopeItem::Type(ScopeTypeItem::new_built_in(
                Type::Function(merkle_tree_map_contains),
                true,
            ))
            .wrap(),
        );
        let merkle_tree_map_insert =
            FunctionType::new_library(LibraryFunctionIdentifier::CollectionsMTreeMapInsert);
        Scope::insert_item(
            merkle_tree_map_scope.clone(),
            merkle_tree_map_insert.identifier(),
            ScopeItem::Type(ScopeTypeItem::new_built_in(
                Type::Function(merkle_tree_map_insert),
                true,
            ))
            .wrap(),
        );
        let merkle_tree_map_remove =
            FunctionType::new_library(LibraryFunctionIdentifier::CollectionsMTreeMapRemove);
        Scope::insert_item(
            merkle_tree_map_scope,
            merkle_tree_map_remove.identifier(),
            ScopeItem::Type(ScopeTypeItem::new_built_in(
                Type::Function(merkle_tree_map_remove),
                true,
            ))
            .wrap(),
        );

        Scope::insert_item(
            scope.clone(),
            merkle_tree_map.identifier.clone(),
            ScopeItem::Type(ScopeTypeItem::new_built_in(
                Type::Structure(merkle_tree_map),
                false,
            ))
            .wrap(),
        );

        scope
    }

    ///
    /// Initializes the `zksync` module scope.
    ///
    fn module_zksync() -> Rc<RefCell<Scope>> {
        let scope = Scope::new_intrinsic("zksync").wrap();

        let transfer = FunctionType::new_library(LibraryFunctionIdentifier::ZksyncTransfer);
        Scope::insert_item(
            scope.clone(),
            transfer.identifier(),
            ScopeItem::Type(ScopeTypeItem::new_built_in(Type::Function(transfer), false)).wrap(),
        );

        let transaction_type = StructureType::new(
            None,
            "Transaction".to_owned(),
            IntrinsicTypeId::ZkSyncTransaction as usize,
            vec![
                (
                    "sender0".to_owned(),
                    Type::integer_unsigned(None, zinc_const::bitlength::ETH_ADDRESS),
                ),
                (
                    "recipient0".to_owned(),
                    Type::integer_unsigned(None, zinc_const::bitlength::ETH_ADDRESS),
                ),
                (
                    "token_address0".to_owned(),
                    Type::integer_unsigned(None, zinc_const::bitlength::ETH_ADDRESS),
                ),
                (
                    "amount0".to_owned(),
                    Type::integer_unsigned(None, zinc_const::bitlength::BALANCE),
                ),
                (
                    "sender1".to_owned(),
                    Type::integer_unsigned(None, zinc_const::bitlength::ETH_ADDRESS),
                ),
                (
                    "recipient1".to_owned(),
                    Type::integer_unsigned(None, zinc_const::bitlength::ETH_ADDRESS),
                ),
                (
                    "token_address1".to_owned(),
                    Type::integer_unsigned(None, zinc_const::bitlength::ETH_ADDRESS),
                ),
                (
                    "amount1".to_owned(),
                    Type::integer_unsigned(None, zinc_const::bitlength::BALANCE),
                ),
            ],
            None,
            None,
            None,
        );

        Scope::insert_item(
            scope.clone(),
            transaction_type.identifier.clone(),
            ScopeItem::Type(ScopeTypeItem::new_built_in(
                Type::Structure(transaction_type.clone()),
                false,
            ))
            .wrap(),
        );

        Scope::insert_item(
            scope.clone(),
            zinc_const::contract::TRANSACTION_VARIABLE_NAME.to_owned(),
            ScopeItem::Variable(ScopeVariableItem::new(
                None,
                false,
                zinc_const::contract::TRANSACTION_VARIABLE_NAME.to_owned(),
                Type::Structure(transaction_type),
                MemoryType::Stack,
            ))
            .wrap(),
        );

        scope
    }
}
