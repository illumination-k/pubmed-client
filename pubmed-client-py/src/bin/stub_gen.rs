//! Stub file generator for pubmed-client Python bindings
//!
//! This binary generates Python type stub files (.pyi) from the Rust source code
//! using pyo3-stub-gen.
//!
//! Usage:
//!     cargo run --bin stub_gen --features stub-gen

use pyo3_stub_gen::Result;

fn main() -> Result<()> {
    // Generate stub information from the library
    let stub = pubmed_client_py::stub_info()?;

    // Generate the .pyi file
    stub.generate()?;

    println!("✓ Generated stub file: pubmed_client.pyi");

    Ok(())
}
