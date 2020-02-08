use crate::commands::read_hex;
use crate::Error;
use colored::Colorize;
use franklin_crypto::bellman::groth16::{Proof, VerifyingKey};
use pairing::bn256::Bn256;
use std::fs;
use std::path::PathBuf;
use std::process::exit;
use structopt::StructOpt;
use zinc_bytecode::data::values::Value;
use zinc_bytecode::Program;

#[derive(Debug, StructOpt)]
pub struct VerifyCommand {
    #[structopt(short = "c", long = "circuit", help = "Compiled circuit program file")]
    pub circuit_path: PathBuf,

    #[structopt(
        short = "k",
        long = "verifying-key",
        about = "Path to verifying key file"
    )]
    pub key_path: PathBuf,

    #[structopt(
        short = "d",
        long = "public-data",
        about = "Path to public data JSON file"
    )]
    pub public_data_path: PathBuf,
}

impl VerifyCommand {
    pub fn execute(&self) -> Result<(), Error> {
        let mut stdin = std::io::stdin();
        let proof_bytes = read_hex(&mut stdin)?;
        let proof = Proof::<Bn256>::read(proof_bytes.as_slice())?;

        let bytes = fs::read(&self.circuit_path)?;
        let program = Program::from_bytes(bytes.as_slice()).unwrap();

        let mut key_file = fs::File::open(&self.key_path)?;
        let key_bytes = read_hex(&mut key_file)?;
        let key = VerifyingKey::<Bn256>::read(key_bytes.as_slice())?;

        let output_text = fs::read_to_string(&self.public_data_path)?;
        let output_value = serde_json::from_str(output_text.as_str())?;
        let output_struct = Value::from_typed_json(&output_value, &program.output)?;
        let output = output_struct.to_flat_values();

        let verified = zinc_vm::verify(&key, &proof, &output)?;

        if verified {
            println!("{}", "✔  Verified".bold().green());
        } else {
            println!("{}", "✘  Failed".bold().red());
            exit(1);
        }

        Ok(())
    }
}