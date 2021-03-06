use std::borrow::BorrowMut;
use std::marker::PhantomData;

use franklin_crypto::bellman::ConstraintSystem;
use franklin_crypto::bellman::SynthesisError;
// use franklin_crypto::circuit::boolean::Boolean;

use zinc_build::ScalarType;

use crate::core::contract::storage::leaf::LeafVariant;
use crate::error::RuntimeError;
use crate::gadgets;
use crate::gadgets::contract::merkle_tree::allocated_leaf::AllocatedLeaf;
use crate::gadgets::contract::merkle_tree::hasher::IHasher as IMerkleTreeHasher;
use crate::gadgets::contract::merkle_tree::IMerkleTree;
use crate::gadgets::scalar::Scalar;
use crate::IEngine;

pub struct StorageGadget<E: IEngine, S: IMerkleTree<E>, H: IMerkleTreeHasher<E>> {
    storage: S,
    root_hash: Scalar<E>,

    _pd: PhantomData<H>,
}

impl<E, S, H> StorageGadget<E, S, H>
where
    E: IEngine,
    S: IMerkleTree<E>,
    H: IMerkleTreeHasher<E>,
{
    pub fn new<CS>(mut cs: CS, storage: S) -> Result<Self, SynthesisError>
    where
        CS: ConstraintSystem<E>,
    {
        let root_hash_value = storage.root_hash();
        let root_hash_variable = cs.alloc(|| "root hash variable", || Ok(root_hash_value))?;
        let root_hash = Scalar::<E>::new_unchecked_variable(
            Some(root_hash_value),
            root_hash_variable,
            ScalarType::Field,
        );

        Ok(StorageGadget {
            storage,
            root_hash,
            _pd: PhantomData,
        })
    }

    pub fn load<CS>(
        &self,
        mut cs: CS,
        _size: usize,
        index: Scalar<E>,
    ) -> Result<Vec<Scalar<E>>, RuntimeError>
    where
        CS: ConstraintSystem<E>,
    {
        let depth = self.storage.depth();
        let mut index_bits = index.get_bits_le(cs.namespace(|| "index into bits"))?;
        index_bits.truncate(depth);

        let index = index
            .get_value()
            .map(|field| gadgets::scalar::fr_bigint::fr_to_bigint::<E>(&field, false))
            .expect(zinc_const::panic::TEST_DATA_VALID);
        let merkle_tree_leaf = self.storage.load(index)?;

        let leaf_value = match merkle_tree_leaf.leaf_values {
            LeafVariant::Array(array) => array,
            LeafVariant::Map { .. } => vec![],
        };
        let leaf_fields =
            AllocatedLeaf::alloc_leaf_fields(cs.namespace(|| "alloc leaf fields"), leaf_value)?;

        // if leaf_fields.len() != size {
        //     return Err(RuntimeError::RequireError(
        //         "Incorrect number of slot fields returned from storage".into(),
        //     ));
        // }

        // let authentication_path = AllocatedLeaf::alloc_authentication_path(
        //     cs.namespace(|| "alloc authentication path"),
        //     depth,
        //     merkle_tree_leaf.authentication_path,
        // )?;
        //
        // let authorized_root_hash = AllocatedLeaf::LeafFields(leaf_fields.clone())
        //     .enforce_merkle_tree_path(
        //         cs.namespace(|| "enforce merkle tree path"),
        //         depth,
        //         &H::default(),
        //         &index_bits,
        //         &authentication_path,
        //     )?;
        //
        // let root_hash_condition = gadgets::comparison::equals(
        //     cs.namespace(|| "root hash equals to stored"),
        //     &authorized_root_hash,
        //     &self.root_hash,
        // )?
        // .to_boolean(cs.namespace(|| "root hash equals to stored to boolean"))?;
        //
        // Boolean::enforce_equal(
        //     cs.namespace(|| "enforcing that root hash equals to stored"),
        //     &root_hash_condition,
        //     &Boolean::Constant(true),
        // )?;

        Ok(leaf_fields)
    }

    pub fn store<CS>(
        &mut self,
        mut cs: CS,
        index: Scalar<E>,
        values: LeafVariant<E>,
    ) -> Result<(), RuntimeError>
    where
        CS: ConstraintSystem<E>,
    {
        let depth = self.storage.depth();
        let mut index_bits = index.get_bits_le(cs.namespace(|| "index into bits"))?;
        index_bits.truncate(depth);

        let _merkle_tree_leaf = self.storage.store(
            index
                .get_value()
                .map(|field| gadgets::scalar::fr_bigint::fr_to_bigint::<E>(&field, false))
                .expect(zinc_const::panic::TEST_DATA_VALID),
            values,
        )?;

        // let leaf_hash = AllocatedLeaf::alloc_leaf_hash(
        //     cs.namespace(|| "alloc leaf hash"),
        //     &merkle_tree_leaf.leaf_value_hash,
        // )?;
        //
        // let authentication_path = AllocatedLeaf::alloc_authentication_path(
        //     cs.namespace(|| "alloc authentication path"),
        //     depth,
        //     merkle_tree_leaf.authentication_path,
        // )?;
        //
        // let authorized_root_hash = AllocatedLeaf::LeafHash(leaf_hash).enforce_merkle_tree_path(
        //     cs.namespace(|| "enforce merkle tree path (loading value)"),
        //     depth,
        //     &H::default(),
        //     &index_bits,
        //     &authentication_path,
        // )?;
        //
        // let root_hash_condition = gadgets::comparison::equals(
        //     cs.namespace(|| "root hash equals to stored"),
        //     &authorized_root_hash,
        //     &self.root_hash,
        // )?
        // .to_boolean(cs.namespace(|| "root hash equals to stored to boolean"))?;
        //
        // Boolean::enforce_equal(
        //     cs.namespace(|| "enforcing that root hash equals to stored"),
        //     &root_hash_condition,
        //     &Boolean::Constant(true),
        // )?;
        //
        // self.root_hash = AllocatedLeaf::LeafFields(values).enforce_merkle_tree_path(
        //     cs.namespace(|| "enforce merkle tree path (storing value)"),
        //     depth,
        //     &H::default(),
        //     &index_bits,
        //     &authentication_path,
        // )?;

        Ok(())
    }

    pub fn root_hash(&self) -> Result<Scalar<E>, RuntimeError> {
        Ok(self.root_hash.clone())
    }

    #[allow(clippy::should_implement_trait)]
    pub fn as_mut(&mut self) -> &mut S {
        self.storage.borrow_mut()
    }

    pub fn into_inner(self) -> S {
        self.storage
    }
}

#[cfg(test)]
mod tests {
    use num::BigInt;
    use num::Zero;
    use rand::Rng;
    use rand::SeedableRng;
    use rand::XorShiftRng;

    use franklin_crypto::bellman::pairing::bn256::Bn256;
    use franklin_crypto::bellman::ConstraintSystem;
    use franklin_crypto::circuit::num::AllocatedNum;
    use franklin_crypto::circuit::test::TestConstraintSystem;

    use zinc_build::ScalarType;
    use zinc_build::Type as BuildType;

    use crate::core::contract::storage::database::Storage as DatabaseStorage;
    use crate::core::contract::storage::leaf::LeafInput;
    use crate::core::contract::storage::leaf::LeafVariant;
    use crate::gadgets::contract::merkle_tree::hasher::sha256::Hasher as Sha256Hasher;
    use crate::gadgets::contract::storage::StorageGadget;
    use crate::gadgets::scalar::Scalar;

    #[test]
    fn test_storage_gadget_small() {
        const STORAGE_ELEMENT_COUNT: usize = 2;

        let mut rng = XorShiftRng::from_seed([0x5dbe6259, 0x8d313d76, 0x3237db17, 0xe5bc0654]);

        let mut cs = TestConstraintSystem::<Bn256>::new();

        let storage = DatabaseStorage::<Bn256>::new(vec![
            LeafInput::Array {
                r#type: BuildType::Scalar(ScalarType::Field),
                values: vec![BigInt::zero()],
            };
            STORAGE_ELEMENT_COUNT
        ]);

        let mut storage_gadget =
            StorageGadget::<_, _, Sha256Hasher>::new(cs.namespace(|| "gadget creation"), storage)
                .expect(zinc_const::panic::TEST_DATA_VALID);

        for i in 0..STORAGE_ELEMENT_COUNT {
            let scalar = Scalar::<Bn256>::from(
                AllocatedNum::alloc(
                    cs.namespace(|| format!("variable :: index({}); field index({})", i, 1)),
                    || Ok(rng.gen()),
                )
                .expect(zinc_const::panic::TEST_DATA_VALID),
            );
            let fr = scalar
                .get_value()
                .expect(zinc_const::panic::TEST_DATA_VALID);

            storage_gadget
                .store(
                    cs.namespace(|| format!("store :: index({})", i)),
                    Scalar::<Bn256>::new_constant_usize(i, ScalarType::Field),
                    LeafVariant::Array(vec![scalar]),
                )
                .expect(zinc_const::panic::TEST_DATA_VALID);

            let loaded_fr = storage_gadget
                .load(
                    cs.namespace(|| format!("load :: index({})", i)),
                    1,
                    Scalar::<Bn256>::new_constant_usize(i, ScalarType::Field),
                )
                .expect(zinc_const::panic::TEST_DATA_VALID)
                .remove(0)
                .get_value()
                .expect(zinc_const::panic::TEST_DATA_VALID);

            assert_eq!(loaded_fr, fr);
        }

        assert!(cs.is_satisfied());
    }
}
