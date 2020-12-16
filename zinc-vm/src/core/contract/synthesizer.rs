//!
//! The virtual machine contract synthesizer.
//!

use std::marker::PhantomData;

use num::BigInt;

use franklin_crypto::bellman;
use franklin_crypto::bellman::ConstraintSystem;
use franklin_crypto::bellman::SynthesisError;

use zinc_build::Contract as BytecodeContract;
use zinc_build::ContractMethod;
use zinc_zksync::TransactionMsg;

use crate::constraint_systems::dedup::Dedup as DedupCS;
use crate::constraint_systems::logging::Logging as LoggingCS;
use crate::core::contract::State;
use crate::error::RuntimeError;
use crate::gadgets::contract::merkle_tree::hasher::sha256::Hasher as Sha256Hasher;
use crate::gadgets::contract::merkle_tree::IMerkleTree;
use crate::gadgets::contract::storage::StorageGadget;
use crate::IEngine;

pub struct Synthesizer<'a, E: IEngine, S: IMerkleTree<E>> {
    pub inputs: Option<Vec<BigInt>>,
    pub output: &'a mut Option<Result<Vec<Option<BigInt>>, RuntimeError>>,
    pub bytecode: BytecodeContract,
    pub method: ContractMethod,
    pub storage: S,
    pub transactions: Vec<TransactionMsg>,

    pub _pd: PhantomData<E>,
}

impl<E, S> bellman::Circuit<E> for Synthesizer<'_, E, S>
where
    E: IEngine,
    S: IMerkleTree<E>,
{
    fn synthesize<CS: ConstraintSystem<E>>(self, cs: &mut CS) -> Result<(), SynthesisError> {
        let storage = StorageGadget::<_, _, Sha256Hasher>::new(
            cs.namespace(|| "storage init"),
            self.storage,
        )?;

        let mut contract = State::new(
            DedupCS::new(LoggingCS::new(cs)),
            storage,
            self.method.name,
            self.transactions,
        );

        *self.output = Some(contract.run(
            self.bytecode,
            self.method.input,
            self.inputs.as_deref(),
            |_| {},
            |_| Ok(()),
            self.method.address,
        ));

        Ok(())
    }
}
