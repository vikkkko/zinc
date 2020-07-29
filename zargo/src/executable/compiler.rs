//!
//! The compiler executable.
//!

use std::io;
use std::path::PathBuf;
use std::process;
use std::process::ExitStatus;

use failure::Fail;

///
/// The Zinc compiler process representation.
///
pub struct Compiler {}

///
/// The Zinc virtual machine process error.
///
#[derive(Debug, Fail)]
pub enum Error {
    /// The process spawning error.
    #[fail(display = "spawning: {}", _0)]
    Spawning(io::Error),
    /// The process waiting error.
    #[fail(display = "waiting: {}", _0)]
    Waiting(io::Error),
    /// The process returned a non-success exit code.
    #[fail(display = "failure: {}", _0)]
    Failure(ExitStatus),
}

impl Compiler {
    ///
    /// Executes the compiler process, building the debug build without optimizations.
    ///
    /// If `is_test_only` is set, passes the flag to only build the project unit tests.
    ///
    pub fn build_debug(
        verbosity: usize,
        data_path: &PathBuf,
        build_path: &PathBuf,
        source_path: &PathBuf,
        is_test_only: bool,
    ) -> Result<(), Error> {
        let mut child = process::Command::new(zinc_const::app_name::ZINC_COMPILER)
            .args(vec!["-v"; verbosity])
            .arg("--data")
            .arg(data_path)
            .arg("--build")
            .arg(build_path)
            .args(if is_test_only {
                vec!["--test-only"]
            } else {
                vec![]
            })
            .arg(source_path)
            .spawn()
            .map_err(Error::Spawning)?;

        let status = child.wait().map_err(Error::Waiting)?;

        if !status.success() {
            return Err(Error::Failure(status));
        }

        Ok(())
    }

    ///
    /// Executes the compiler process, building the release build with optimizations.
    ///
    /// If `is_test_only` is set, passes the flag to only build the project unit tests.
    ///
    pub fn build_release(
        verbosity: usize,
        data_path: &PathBuf,
        build_path: &PathBuf,
        source_path: &PathBuf,
        is_test_only: bool,
    ) -> Result<(), Error> {
        let mut child = process::Command::new(zinc_const::app_name::ZINC_COMPILER)
            .args(vec!["-v"; verbosity])
            .arg("--data")
            .arg(data_path)
            .arg("--build")
            .arg(build_path)
            .args(if is_test_only {
                vec!["--test-only"]
            } else {
                vec![]
            })
            .arg("--optimize-dead-function-elimination")
            .arg(source_path)
            .spawn()
            .map_err(Error::Spawning)?;

        let status = child.wait().map_err(Error::Waiting)?;

        if !status.success() {
            return Err(Error::Failure(status));
        }

        Ok(())
    }
}
