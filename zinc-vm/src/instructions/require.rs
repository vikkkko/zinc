//!
//! The `Require` instruction.
//!

use franklin_crypto::bellman::ConstraintSystem;

use zinc_build::Require;

use crate::core::virtual_machine::IVirtualMachine;
use crate::error::RuntimeError;
use crate::gadgets;
use crate::instructions::IExecutable;

impl<VM: IVirtualMachine> IExecutable<VM> for Require {
    fn execute(self, vm: &mut VM) -> Result<(), RuntimeError> {
        let value = vm.pop()?.try_into_value()?;
        let condition = vm.condition_top()?;

        let cs = vm.constraint_system();

        let not_c = gadgets::logical::not::not(cs.namespace(|| "not"), &condition)?;
        let condition = gadgets::logical::or::or(cs.namespace(|| "or"), &value, &not_c)?;

        let message = match &self.message {
            Some(message) => Some(message.as_str()),
            None => None,
        };
        gadgets::require::require(cs, condition, message)
    }
}

#[cfg(test)]
mod tests {
    use num::BigInt;
    use num::One;
    use num::Zero;

    use crate::error::RuntimeError;
    use crate::tests::TestRunner;
    use crate::tests::TestingError;

    use zinc_build::ScalarType;

    #[test]
    fn test_require_ok() -> Result<(), TestingError> {
        TestRunner::new()
            .push(zinc_build::Push::new(BigInt::one(), ScalarType::Boolean))
            .push(zinc_build::Require::new(None))
            .test::<i32>(&[])
    }

    #[test]
    fn test_require_fail() {
        let res = TestRunner::new()
            .push(zinc_build::Push::new(BigInt::zero(), ScalarType::Boolean))
            .push(zinc_build::Require::new(None))
            .test::<i32>(&[]);

        match res {
            Err(TestingError::RuntimeError(RuntimeError::RequireError(_))) => {}
            _ => panic!("Expected require error"),
        }
    }

    #[test]
    fn test_require_in_condition() -> Result<(), TestingError> {
        TestRunner::new()
            .push(zinc_build::Push::new(BigInt::zero(), ScalarType::Boolean))
            .push(zinc_build::If)
            .push(zinc_build::Push::new(BigInt::zero(), ScalarType::Boolean))
            .push(zinc_build::Require::new(None))
            .push(zinc_build::EndIf)
            .test::<i32>(&[])
    }
}
