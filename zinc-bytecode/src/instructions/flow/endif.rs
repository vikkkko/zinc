use crate::instructions::utils::decode_simple_instruction;
use crate::{DecodingError, Instruction, InstructionCode, InstructionInfo};
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Default, Clone, Serialize, Deserialize)]
pub struct EndIf;

impl InstructionInfo for EndIf {
    fn to_assembly(&self) -> String {
        "endif".into()
    }

    fn code() -> InstructionCode {
        InstructionCode::EndIf
    }

    fn encode(&self) -> Vec<u8> {
        vec![InstructionCode::EndIf as u8]
    }

    fn decode(bytes: &[u8]) -> Result<(Self, usize), DecodingError> {
        decode_simple_instruction(bytes)
    }

    fn inputs_count(&self) -> usize {
        0
    }

    fn outputs_count(&self) -> usize {
        0
    }

    fn wrap(&self) -> Instruction {
        Instruction::EndIf((*self).clone())
    }
}
