use pyo3_stub_gen::Result;

fn main() -> Result<()> {
    let stub = pubmed_client_py::stub_info()?;
    stub.generate()?;
    Ok(())
}
