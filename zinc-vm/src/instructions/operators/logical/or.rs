//!
//! The `Or` instruction.
//!

use num::bigint::ToBigInt;
use num::One;

use zinc_build::Or;

use crate::core::execution_state::cell::Cell;
use crate::core::virtual_machine::IVirtualMachine;
use crate::error::RuntimeError;
use crate::gadgets::scalar::Scalar;
use crate::instructions::IExecutable;

impl<VM: IVirtualMachine> IExecutable<VM> for Or {
    fn execute(self, vm: &mut VM) -> Result<(), RuntimeError> {
        let right = vm.pop()?.try_into_value()?.to_bigint().unwrap_or_default();
        let left = vm.pop()?.try_into_value()?.to_bigint().unwrap_or_default();

        vm.push(Cell::Value(Scalar::new_constant_bool(
            left.is_one() || right.is_one(),
        )))
    }
}

#[cfg(test)]
mod tests {
    use num::BigInt;
    use num::One;
    use num::Zero;

    use zinc_build::ScalarType;

    use crate::tests::TestRunner;
    use crate::tests::TestingError;

    #[test]
    fn test_or() -> Result<(), TestingError> {
        TestRunner::new()
            .push(zinc_build::Push::new(BigInt::zero(), ScalarType::Boolean))
            .push(zinc_build::Push::new(BigInt::zero(), ScalarType::Boolean))
            .push(zinc_build::Or)
            .push(zinc_build::Push::new(BigInt::zero(), ScalarType::Boolean))
            .push(zinc_build::Push::new(BigInt::one(), ScalarType::Boolean))
            .push(zinc_build::Or)
            .push(zinc_build::Push::new(BigInt::one(), ScalarType::Boolean))
            .push(zinc_build::Push::new(BigInt::zero(), ScalarType::Boolean))
            .push(zinc_build::Or)
            .push(zinc_build::Push::new(BigInt::one(), ScalarType::Boolean))
            .push(zinc_build::Push::new(BigInt::one(), ScalarType::Boolean))
            .push(zinc_build::Or)
            .test(&[1, 1, 1, 0])
    }
}
