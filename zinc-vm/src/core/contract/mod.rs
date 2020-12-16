//!
//! The virtual machine contract.
//!

pub mod facade;
pub mod input;
pub mod output;
pub mod storage;
pub mod synthesizer;

use colored::Colorize;
use num::bigint::Sign;
use num::bigint::ToBigInt;
use num::BigInt;

use franklin_crypto::bellman::ConstraintSystem;

use zinc_build::Contract as BytecodeContract;
use zinc_build::IntegerType;
use zinc_build::ScalarType;
use zinc_build::Type as BuildType;
use zinc_zksync::TransactionMsg;

use crate::core::contract::storage::leaf::LeafVariant;
use crate::core::counter::NamespaceCounter;
use crate::core::execution_state::block::branch::Branch;
use crate::core::execution_state::block::r#loop::Loop;
use crate::core::execution_state::block::Block;
use crate::core::execution_state::cell::Cell;
use crate::core::execution_state::function_frame::Frame;
use crate::core::execution_state::ExecutionState;
use crate::core::location::Location;
use crate::core::virtual_machine::IVirtualMachine;
use crate::error::MalformedBytecode;
use crate::error::RuntimeError;
use crate::gadgets;
use crate::gadgets::contract::merkle_tree::hasher::IHasher as IMerkleTreeHasher;
use crate::gadgets::contract::merkle_tree::IMerkleTree;
use crate::gadgets::contract::storage::StorageGadget;
use crate::gadgets::scalar::Scalar;
use crate::instructions::call_library::INativeCallable;
use crate::instructions::IExecutable;
use crate::IEngine;

pub struct State<E, CS, S, H>
where
    E: IEngine,
    CS: ConstraintSystem<E>,
    S: IMerkleTree<E>,
    H: IMerkleTreeHasher<E>,
{
    counter: NamespaceCounter<E, CS>,
    execution_state: ExecutionState<E>,
    outputs: Vec<Scalar<E>>,

    storage: StorageGadget<E, S, H>,
    method_name: String,
    transactions: Vec<TransactionMsg>,

    pub(crate) location: Location,
}

impl<E, CS, S, H> State<E, CS, S, H>
where
    E: IEngine,
    CS: ConstraintSystem<E>,
    S: IMerkleTree<E>,
    H: IMerkleTreeHasher<E>,
{
    pub fn new(
        cs: CS,
        storage: StorageGadget<E, S, H>,
        method_name: String,
        transactions: Vec<TransactionMsg>,
    ) -> Self {
        Self {
            counter: NamespaceCounter::new(cs),
            execution_state: ExecutionState::new(),
            outputs: vec![],

            storage,
            method_name,
            transactions,

            location: Location::new(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn run<CB, F>(
        &mut self,
        contract: BytecodeContract,
        input_type: BuildType,
        input_values: Option<&[BigInt]>,
        mut instruction_callback: CB,
        mut check_cs: F,
        address: usize,
    ) -> Result<Vec<Option<BigInt>>, RuntimeError>
    where
        CB: FnMut(&CS),
        F: FnMut(&CS) -> Result<(), RuntimeError>,
    {
        self.counter.cs.enforce(
            || "ONE * ONE = ONE (do this to avoid `unconstrained` error)",
            |zero| zero + CS::one(),
            |zero| zero + CS::one(),
            |zero| zero + CS::one(),
        );
        let one = Scalar::new_constant_usize(1, ScalarType::Boolean);
        self.condition_push(one)?;

        let input_size = input_type.size();
        self.execution_state
            .frames_stack
            .push(Frame::new(0, std::usize::MAX));
        self.init_root_frame(input_type, input_values)?;

        if let Err(error) = zinc_build::Call::new(address, input_size)
            .execute(self)
            .and(check_cs(&self.counter.cs))
        {
            log::error!("{}\nat {}", error, self.location.to_string().blue());
            return Err(error);
        }
        self.init_storage()?;

        let mut step = 0;
        let execution_time = std::time::Instant::now();
        while self.execution_state.instruction_counter < contract.instructions.len() {
            let namespace = format!(
                "step={}, addr={}",
                step, self.execution_state.instruction_counter
            );
            self.counter.cs.push_namespace(|| namespace);
            let instruction =
                contract.instructions[self.execution_state.instruction_counter].clone();

            log::trace!(
                "{}:{} > {}",
                step,
                self.execution_state.instruction_counter,
                instruction,
            );

            self.execution_state.instruction_counter += 1;
            log::debug!("instruction,{:?}",instruction);
            if let Err(error) = instruction.execute(self).and(check_cs(&self.counter.cs)) {
                log::error!("{}\nat {}", error, self.location.to_string().blue());
                return Err(error);
            }

            log::trace!("{}", self.execution_state);
            instruction_callback(&self.counter.cs);
            self.counter.cs.pop_namespace();
            step += 1;
        }

        log::trace!(
            "Elapsed time: {} micros",
            execution_time.elapsed().as_micros()
        );

        self.get_outputs()
    }

    fn init_storage(&mut self) -> Result<(), RuntimeError> {
        // Temporary fix to avoid "unconstrained" error
        let root_hash = self.storage.root_hash()?;
        let cs = self.constraint_system();

        gadgets::arithmetic::add::add(
            cs.namespace(|| "root_hash constraint"),
            &root_hash,
            &Scalar::new_constant_usize(0, ScalarType::Field),
        )?;

        Ok(())
    }

    fn init_root_frame(
        &mut self,
        input_type: BuildType,
        inputs: Option<&[BigInt]>,
    ) -> Result<(), RuntimeError> {
        self.execution_state
            .frames_stack
            .push(Frame::new(0, std::usize::MAX));

        let types = input_type.into_flat_scalar_types();

        // Convert Option<&[BigInt]> to iterator of Option<&BigInt> and zip with types.
        let value_type_pairs: Vec<_> = match inputs {
            Some(values) => values.iter().map(Option::Some).zip(types).collect(),
            None => std::iter::repeat(None).zip(types).collect(),
        };

        for (value, dtype) in value_type_pairs {
            let variable = gadgets::witness::allocate(self.counter.next(), value, dtype)?;
            self.push(Cell::Value(variable))?;
        }

        Ok(())
    }

    fn get_outputs(&mut self) -> Result<Vec<Option<BigInt>>, RuntimeError> {
        let outputs_fr: Vec<_> = self.outputs.iter().map(|f| (*f).clone()).collect();

        let mut outputs_bigint = Vec::with_capacity(outputs_fr.len());
        for output in outputs_fr.into_iter() {
            let output = gadgets::output::output(self.counter.next(), output)?;
            outputs_bigint.push(output.to_bigint());
        }
        let root_hash = self.storage.root_hash()?;
        let root_hash = gadgets::output::output(self.counter.next(), root_hash)?;
        outputs_bigint.push(root_hash.to_bigint());

        Ok(outputs_bigint)
    }

    pub fn condition_push(&mut self, element: Scalar<E>) -> Result<(), RuntimeError> {
        self.execution_state.conditions_stack.push(element);
        Ok(())
    }

    pub fn condition_pop(&mut self) -> Result<Scalar<E>, RuntimeError> {
        self.execution_state
            .conditions_stack
            .pop()
            .ok_or_else(|| MalformedBytecode::StackUnderflow.into())
    }

    fn top_frame(&mut self) -> Result<&mut Frame<E>, RuntimeError> {
        self.execution_state
            .frames_stack
            .last_mut()
            .ok_or_else(|| MalformedBytecode::StackUnderflow.into())
    }
}

impl<E, CS, S, H> IVirtualMachine for State<E, CS, S, H>
where
    E: IEngine,
    CS: ConstraintSystem<E>,
    S: IMerkleTree<E>,
    H: IMerkleTreeHasher<E>,
{
    type E = E;
    type CS = CS;
    type S = S;

    fn push(&mut self, cell: Cell<E>) -> Result<(), RuntimeError> {
        self.execution_state.evaluation_stack.push(cell)
    }

    fn pop(&mut self) -> Result<Cell<E>, RuntimeError> {
        self.execution_state.evaluation_stack.pop()
    }

    fn load(&mut self, address: usize) -> Result<Cell<E>, RuntimeError> {
        let frame_start = self.top_frame()?.stack_frame_start;
        log::debug!("address:{:?}-------frame_start:{:?}",address,frame_start);
        self.execution_state.data_stack.get(frame_start + address)
    }

    fn store(&mut self, address: usize, cell: Cell<E>) -> Result<(), RuntimeError> {
        let frame = self.top_frame()?;
        frame.stack_frame_end =
            std::cmp::max(frame.stack_frame_end, frame.stack_frame_start + address + 1);

        let frame_start = frame.stack_frame_start;
        log::debug!("frame_start:{:?}-------address:{:?}",frame_start,address);

        self.execution_state
            .data_stack
            .set(frame_start + address, cell)
    }

    fn storage_load(
        &mut self,
        index: Scalar<Self::E>,
        size: usize,
    ) -> Result<Vec<Scalar<Self::E>>, RuntimeError> {
        self.storage.load(self.counter.next(), size, index)
    }

    fn storage_store(
        &mut self,
        index: Scalar<Self::E>,
        values: LeafVariant<Self::E>,
    ) -> Result<(), RuntimeError> {
        self.storage.store(self.counter.next(), index, values)
    }

    fn loop_begin(&mut self, iterations: usize) -> Result<(), RuntimeError> {
        let frame = self
            .execution_state
            .frames_stack
            .last_mut()
            .ok_or_else(|| RuntimeError::InternalError("Root frame is missing".into()))?;

        frame.blocks.push(Block::Loop(Loop {
            first_instruction_index: self.execution_state.instruction_counter,
            iterations_left: iterations - 1,
        }));

        Ok(())
    }

    fn loop_end(&mut self) -> Result<(), RuntimeError> {
        let frame = self
            .execution_state
            .frames_stack
            .last_mut()
            .expect(zinc_const::panic::VALUE_ALWAYS_EXISTS);

        match frame.blocks.pop() {
            Some(Block::Loop(mut loop_block)) => {
                if loop_block.iterations_left != 0 {
                    loop_block.iterations_left -= 1;
                    self.execution_state.instruction_counter = loop_block.first_instruction_index;
                    frame.blocks.push(Block::Loop(loop_block));
                }
                Ok(())
            }
            _ => Err(MalformedBytecode::UnexpectedLoopEnd.into()),
        }
    }

    fn call(&mut self, address: usize, inputs_count: usize) -> Result<(), RuntimeError> {
        let offset = self.top_frame()?.stack_frame_end;
        self.execution_state
            .frames_stack
            .push(Frame::new(offset, self.execution_state.instruction_counter));
        let tran_len = self.transactions.len();    
        let mut transaction_field_iter = 0..4*tran_len;

        for i in 0..self.transactions.len() {
            let sender : [u8; zinc_const::size::ETH_ADDRESS] = (&self.transactions[i]).sender.into();
            let sender = gadgets::witness::allocate(
                self.counter.next(),
                Some(&BigInt::from_bytes_be(
                    Sign::Plus,
                    sender.to_vec().as_slice(),
                )),
                ScalarType::Integer(IntegerType::ETH_ADDRESS),
            )?;
            log::debug!("sender========================={:?}", sender.clone());
    
            self.store(
                transaction_field_iter
                    .next()
                    .expect(zinc_const::panic::VALUE_ALWAYS_EXISTS),
                Cell::Value(sender),
            )?;
    
            let recipient: [u8; zinc_const::size::ETH_ADDRESS] =
                (&self.transactions[i]).recipient.into();
            let recipient = gadgets::witness::allocate(
                self.counter.next(),
                Some(&BigInt::from_bytes_be(
                    Sign::Plus,
                    recipient.to_vec().as_slice(),
                )),
                ScalarType::Integer(IntegerType::ETH_ADDRESS),
            )?;
            log::debug!("recipient========================={:?}", recipient.clone());
    
            self.store(
                transaction_field_iter
                    .next()
                    .expect(zinc_const::panic::VALUE_ALWAYS_EXISTS),
                Cell::Value(recipient),
            )?;
    
            let token_address: [u8; zinc_const::size::ETH_ADDRESS] =
                (&self.transactions[i]).token_address.into();
            let token_address = gadgets::witness::allocate(
                self.counter.next(),
                Some(&BigInt::from_bytes_be(
                    Sign::Plus,
                    token_address.to_vec().as_slice(),
                )),
                ScalarType::Integer(IntegerType::ETH_ADDRESS),
            )?;
    
            log::debug!("token_address========================={:?}", token_address.clone());
    
            self.store(
                transaction_field_iter
                    .next()
                    .expect(zinc_const::panic::VALUE_ALWAYS_EXISTS),
                Cell::Value(token_address),
            )?;
    
            let amount = gadgets::witness::allocate(
                self.counter.next(),
                Some(
                    &zinc_zksync::num_compat_forward((&self.transactions[i]).amount.to_owned())
                        .to_bigint()
                        .expect(zinc_const::panic::DATA_CONVERSION),
                ),
                ScalarType::Integer(IntegerType::BALANCE),
            )?;
    
            log::debug!("amount========================={:?}", amount.clone());
    
            self.store(
                transaction_field_iter
                    .next()
                    .expect(zinc_const::panic::VALUE_ALWAYS_EXISTS),
                Cell::Value(amount),
            )?;
        }
        
        log::debug!("inputs_count========================={:?}", inputs_count);

        for i in 0..inputs_count {
            let arg = self.pop()?;
            log::debug!("arg========================={:?}", arg);
            self.store(
                zinc_const::contract::TRANSACTION_SIZE + inputs_count - i - 1,
                arg,
            )?;
        }

        self.execution_state.instruction_counter = address;
        Ok(())
    }

    fn r#return(&mut self, outputs_count: usize) -> Result<(), RuntimeError> {
        let mut outputs = Vec::new();
        for _ in 0..outputs_count {
            let output = self.pop()?;
            outputs.push(output);
        }

        let frame = self
            .execution_state
            .frames_stack
            .pop()
            .ok_or(MalformedBytecode::StackUnderflow)?;

        self.execution_state.instruction_counter = frame.return_address;

        self.execution_state
            .data_stack
            .drop_from(frame.stack_frame_start);

        for p in outputs.into_iter().rev() {
            self.push(p)?;
        }

        Ok(())
    }

    fn branch_then(&mut self) -> Result<(), RuntimeError> {
        let condition = self.pop()?.try_into_value()?;

        let prev = self.condition_top()?;

        let cs = self.constraint_system();
        let next = gadgets::logical::and::and(cs.namespace(|| "branch"), &condition, &prev)?;
        self.execution_state.conditions_stack.push(next);

        let branch = Branch {
            condition,
            is_else: false,
        };

        self.top_frame()?.blocks.push(Block::Branch(branch));

        self.execution_state.evaluation_stack.fork();
        self.execution_state.data_stack.fork();

        Ok(())
    }

    fn branch_else(&mut self) -> Result<(), RuntimeError> {
        let frame = self
            .execution_state
            .frames_stack
            .last_mut()
            .ok_or_else(|| RuntimeError::InternalError("Root frame is missing".into()))?;

        let mut branch = match frame.blocks.pop() {
            Some(Block::Branch(branch)) => Ok(branch),
            Some(_) | None => Err(RuntimeError::MalformedBytecode(
                MalformedBytecode::UnexpectedElse,
            )),
        }?;

        if branch.is_else {
            return Err(MalformedBytecode::UnexpectedElse.into());
        } else {
            branch.is_else = true;
        }

        let condition = branch.condition.clone();

        frame.blocks.push(Block::Branch(branch));

        self.condition_pop()?;
        let prev = self.condition_top()?;
        let not_cond = gadgets::logical::not::not(self.counter.next(), &condition)?;
        let next = gadgets::logical::and::and(self.counter.next(), &prev, &not_cond)?;
        self.condition_push(next)?;

        self.execution_state.data_stack.switch_branch()?;
        self.execution_state.evaluation_stack.fork();

        Ok(())
    }

    fn branch_end(&mut self) -> Result<(), RuntimeError> {
        self.condition_pop()?;

        let frame = self
            .execution_state
            .frames_stack
            .last_mut()
            .ok_or_else(|| RuntimeError::InternalError("Root frame is missing".into()))?;

        let branch = match frame.blocks.pop() {
            Some(Block::Branch(branch)) => Ok(branch),
            Some(_) | None => Err(MalformedBytecode::UnexpectedEndIf),
        }?;

        if branch.is_else {
            self.execution_state
                .evaluation_stack
                .merge(self.counter.next(), &branch.condition)?;
        } else {
            self.execution_state.evaluation_stack.revert()?;
        }

        self.execution_state
            .data_stack
            .merge(self.counter.next(), branch.condition)?;

        Ok(())
    }

    fn exit(&mut self, mut outputs_count: usize) -> Result<(), RuntimeError> {
        if self.method_name.as_str() == zinc_const::contract::CONSTRUCTOR_NAME {
            outputs_count -= zinc_const::contract::IMPLICIT_FIELDS_SIZE;
        }

        for _ in 0..outputs_count {
            let value = self.pop()?.try_into_value()?;
            self.outputs.push(value);
        }

        if self.method_name.as_str() == zinc_const::contract::CONSTRUCTOR_NAME {
            self.outputs.extend(
                vec![Scalar::new_constant_usize(
                    0,
                    ScalarType::Integer(IntegerType::ETH_ADDRESS),
                )]
                .into_iter()
                .rev(),
            );
        }
        self.outputs.reverse();

        self.execution_state.instruction_counter = std::usize::MAX;
        Ok(())
    }

    fn call_native<F: INativeCallable<E, S>>(&mut self, function: F) -> Result<(), RuntimeError> {
        let state = &mut self.execution_state;
        let cs = &mut self.counter.cs;

        function.call(
            cs.namespace(|| "native function"),
            state,
            Some(self.storage.as_mut()),
        )
    }

    fn condition_top(&mut self) -> Result<Scalar<E>, RuntimeError> {
        self.execution_state
            .conditions_stack
            .last()
            .map(|e| (*e).clone())
            .ok_or_else(|| MalformedBytecode::StackUnderflow.into())
    }

    fn constraint_system(&mut self) -> &mut CS {
        &mut self.counter.cs
    }

    fn get_location(&mut self) -> Location {
        self.location.clone()
    }

    fn set_location(&mut self, location: Location) {
        self.location = location;
    }
}
