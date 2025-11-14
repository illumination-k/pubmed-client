# Research: Automating Stub Generation for PyO3/Maturin Projects

**Date**: 2025-11-14
**Project**: pubmed-client-py
**Current Status**: Manual `.pyi` maintenance

## Executive Summary

This research investigates tools and approaches for automating Python type stub (`.pyi`) generation for our PyO3-based Python bindings. Currently, we manually maintain a 224-line `pubmed_client.pyi` file that must be kept in sync with 1144+ lines of Rust code in `src/lib.rs`.

**Key Finding**: Multiple viable solutions exist, ranging from fully automated (with limitations) to semi-automated (with more control). The best approach depends on our maintenance priorities and type annotation requirements.

---

## Current State

### What We Have

- **Manual stub file**: `pubmed_client.pyi` (224 lines)
- **Rust bindings**: `src/lib.rs` (1144+ lines across multiple modules)
- **23 PyO3 classes**: PubMedClient, PmcClient, SearchQuery, various data models
- **Complex types**: Lists, Options, Union types, builder patterns

### Pain Points

1. **Manual synchronization** required after adding new methods/classes
2. **Known maturin caching issue** where new methods don't export properly
3. **Duplicate documentation** - docstrings in both Rust and stub files
4. **Human error risk** - easy to forget updating stubs

---

## Available Solutions

### 1. **pyo3-stub-gen** (Rust Crate) ⭐ RECOMMENDED

**Project**: [Jij-Inc/pyo3-stub-gen](https://github.com/Jij-Inc/pyo3-stub-gen)
**Approach**: Compile-time stub generation via procedural macros
**Maturity**: Active development, v0.17+ (as of Nov 2025)

#### How It Works

Add procedural macros to your Rust code:

```rust
use pyo3_stub_gen::derive::{gen_stub_pyfunction, gen_stub_pyclass};

#[gen_stub_pyclass]
#[pyclass]
pub struct PyPubMedClient {
    // ...
}

#[gen_stub_pymethods]
#[pymethods]
impl PyPubMedClient {
    #[gen_stub_pyfunction]
    pub fn search_articles(&self, query: String, limit: usize) -> PyResult<Vec<String>> {
        // ...
    }
}
```

Then create a stub generation binary in `src/bin/stub_gen.rs`:

```rust
use pyo3_stub_gen::Result;

fn main() -> Result<()> {
    let stub = pubmed_client_py::stub_info()?;
    stub.generate()?;
    Ok(())
}
```

Generate stubs: `cargo run --bin stub_gen`

#### Pros

- ✅ **Integrated with source code** - stubs stay close to implementation
- ✅ **Type-aware** - uses Rust type information directly
- ✅ **Automatic import extraction** - figures out needed Python imports
- ✅ **Manual override support** - can specify custom types for complex cases
- ✅ **Maturin integration** - automatically packages `.pyi` files
- ✅ **Active maintenance** - regular updates

#### Cons

- ❌ **Code annotation overhead** - requires adding macros to every function/class
- ❌ **Semi-automated** - some manual type mappings still needed
- ❌ **Build complexity** - adds `rlib` crate type requirement
- ❌ **Learning curve** - need to understand override mechanisms

#### Manual Override Example

For complex types that don't map cleanly:

```rust
#[gen_stub_pyfunction(python = r#"
def search_and_fetch(
    self,
    query: str | SearchQuery,
    limit: int
) -> list[PubMedArticle]: ...
"#)]
pub fn search_and_fetch(
    &self,
    query: PyObject,  // Accepts str or SearchQuery
    limit: usize
) -> PyResult<Vec<PyPubMedArticle>> {
    // ...
}
```

---

### 2. **pyo3-stubgen** (Python Package)

**Project**: [pyo3-stubgen on PyPI](https://pypi.org/project/pyo3-stubgen/)
**Approach**: Runtime inspection of compiled modules
**Maturity**: Basic functionality, limited class support

#### How It Works

After building with maturin:

```bash
maturin develop
pyo3-stubgen pubmed_client -o .
```

#### Pros

- ✅ **No code changes** - works with existing bindings
- ✅ **Simple workflow** - just run after building
- ✅ **No compile-time overhead**

#### Cons

- ❌ **Limited class support** - currently focuses on functions
- ❌ **Requires `__text_signature__`** - depends on PyO3 runtime metadata
- ❌ **Less control** - limited customization options
- ❌ **Post-build step** - manual workflow integration needed

---

### 3. **mypy stubgen** (General Purpose)

**Project**: Built into mypy
**Approach**: Runtime introspection of C extensions
**Maturity**: Stable but generic

#### How It Works

```bash
maturin develop
stubgen -p pubmed_client --include-docstrings -o .
```

#### Pros

- ✅ **No dependencies** - comes with mypy
- ✅ **Works with any C extension**
- ✅ **Include docstrings** option

#### Cons

- ❌ **Generic output** - not PyO3-aware
- ❌ **Limited type information** - relies on runtime signatures
- ❌ **Manual workflow** - no build integration
- ❌ **May miss PyO3-specific patterns**

---

### 4. **Manual Maintenance** (Current Approach)

**Approach**: Hand-written `.pyi` files
**Maturity**: Traditional Python packaging approach

#### Pros

- ✅ **Complete control** - exact types you want
- ✅ **Simple tooling** - just a text file
- ✅ **Well understood** - standard Python practice

#### Cons

- ❌ **Manual synchronization** - high maintenance burden
- ❌ **Error prone** - easy to forget updates
- ❌ **Duplicate documentation**
- ❌ **No automation**

---

## Comparison Matrix

| Feature               | pyo3-stub-gen    | pyo3-stubgen | mypy stubgen | Manual     |
| --------------------- | ---------------- | ------------ | ------------ | ---------- |
| **Automation Level**  | High (semi-auto) | Medium       | Medium       | None       |
| **Type Accuracy**     | ⭐⭐⭐⭐⭐       | ⭐⭐⭐       | ⭐⭐         | ⭐⭐⭐⭐⭐ |
| **Setup Effort**      | High             | Low          | Low          | Low        |
| **Maintenance**       | Low              | Medium       | Medium       | High       |
| **PyO3 Integration**  | ⭐⭐⭐⭐⭐       | ⭐⭐⭐⭐     | ⭐⭐         | ⭐⭐⭐     |
| **Code Changes**      | Required         | None         | None         | None       |
| **Build Integration** | Excellent        | Manual       | Manual       | Manual     |
| **Class Support**     | Full             | Limited      | Full         | Full       |
| **Generic Types**     | ⭐⭐⭐⭐         | ⭐⭐         | ⭐⭐         | ⭐⭐⭐⭐⭐ |
| **Overload Support**  | Auto             | Limited      | Limited      | Manual     |

---

## Recommendations

### For This Project: **Phased Approach**

#### Phase 1: Quick Win with Runtime Tools (Week 1)

Start with **mypy stubgen** or **pyo3-stubgen** to:

1. Generate baseline stubs automatically
2. Compare with current manual stubs
3. Identify gaps and areas needing manual specification

**Workflow**:

```bash
# In pubmed-client-py/
uv run --with maturin maturin develop
uv run stubgen -p pubmed_client --include-docstrings -o stubgen_output/
# Compare: diff pubmed_client.pyi stubgen_output/pubmed_client.pyi
```

#### Phase 2: Evaluate pyo3-stub-gen (Week 2-3)

If runtime tools don't provide sufficient quality:

1. Set up pyo3-stub-gen in a branch
2. Add macros to 2-3 representative classes (e.g., PubMedClient, SearchQuery)
3. Compare generated vs. manual stubs
4. Assess annotation overhead vs. maintenance savings

**Decision Criteria**:

- If generated stubs match 90%+ of manual quality → adopt pyo3-stub-gen
- If significant manual overrides needed → consider hybrid approach
- If annotation overhead too high → stick with improved manual process

#### Phase 3: Automation Integration (Week 4)

Depending on Phase 2 outcome:

**Option A: Full pyo3-stub-gen adoption**

- Add macros to all classes
- Create `src/bin/stub_gen.rs`
- Add CI check: `cargo run --bin stub_gen && git diff --exit-code`
- Update CLAUDE.md with new workflow

**Option B: Hybrid approach**

- Use runtime tools for initial generation
- Manual refinement for complex types
- Add pre-commit hook: `pyo3-stubgen pubmed_client -o .`
- Update CLAUDE.md with verification steps

**Option C: Enhanced manual process**

- Document stub update checklist in CLAUDE.md
- Create template for new classes
- Add CI check comparing class count in .rs vs .pyi

---

## Implementation Guide: pyo3-stub-gen

If we choose the recommended solution, here's the implementation plan:

### 1. Add Dependencies

```toml
# Cargo.toml (workspace)
[workspace.dependencies]
pyo3-stub-gen = "0.17"
pyo3-stub-gen-derive = "0.17"

# pubmed-client-py/Cargo.toml
[dependencies]
pyo3-stub-gen = { workspace = true }
pyo3-stub-gen-derive = { workspace = true }

[lib]
crate-type = ["cdylib", "rlib"] # Add rlib for stub generation binary

[[bin]]
name = "stub_gen"
path = "src/bin/stub_gen.rs"
required-features = ["stub-gen"]

[features]
stub-gen = ["pyo3-stub-gen"]
```

### 2. Create Stub Generation Binary

```rust
// pubmed-client-py/src/bin/stub_gen.rs
use pyo3_stub_gen::Result;

fn main() -> Result<()> {
    let stub = pubmed_client_py::stub_info()?;
    stub.generate()?;
    Ok(())
}
```

### 3. Update Main Module

```rust
// pubmed-client-py/src/lib.rs
#[cfg(feature = "stub-gen")]
use pyo3_stub_gen::{define_stub_info_gatherer, derive::*};

#[cfg(feature = "stub-gen")]
define_stub_info_gatherer!(stub_info);
```

### 4. Annotate Classes (Example)

```rust
// pubmed-client-py/src/client.rs
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

#[gen_stub_pyclass]
#[pyclass(name = "Client")]
pub struct PyClient {
    // ...
}

#[gen_stub_pymethods]
#[pymethods]
impl PyClient {
    #[new]
    pub fn new() -> PyResult<Self> {
        // ...
    }

    // For complex Union types, use manual override:
    #[gen_stub_pyfunction(python = r#"
    def search_and_fetch(
        self,
        query: str | SearchQuery,
        limit: int
    ) -> list[PubMedArticle]: ...
    "#)]
    pub fn search_and_fetch(
        &self,
        py: Python<'_>,
        query: PyObject,
        limit: usize,
    ) -> PyResult<Vec<PyPubMedArticle>> {
        // ...
    }
}
```

### 5. Update Build Process

```bash
# Generate stubs
cargo run --bin stub_gen --features stub-gen

# Verify
cat pubmed_client.pyi

# Build package
uv run --with maturin maturin develop
```

### 6. Add CI Checks

```yaml
# .github/workflows/python-tests.yml
- name: Check stub files are up to date
  run: |
    cargo run --bin stub_gen --features stub-gen
    git diff --exit-code pubmed-client-py/pubmed_client.pyi
```

---

## Potential Issues & Solutions

### Issue 1: Complex Union Types

**Problem**: `Union[str, SearchQuery]` not auto-generated correctly

**Solution**: Use manual override

```rust
#[gen_stub_pyfunction(python = "def foo(x: str | SearchQuery) -> None: ...")]
```

### Issue 2: Optional Collections

**Problem**: `Option<Vec<T>>` vs `list[T] | None`

**Solution**: pyo3-stub-gen handles this automatically, but verify with test

### Issue 3: Property Methods

**Problem**: Rust methods that should be Python properties

**Solution**: Use `#[pyo3(name = "...")]` and document in manual override if needed

### Issue 4: Nested Modules

**Problem**: Sub-modules not organized correctly

**Solution**: Use pyo3-stub-gen's `submit!` macro for complex module structures

---

## Migration Checklist

If adopting pyo3-stub-gen:

- [ ] Add pyo3-stub-gen dependencies to Cargo.toml
- [ ] Add `rlib` to crate-type in pubmed-client-py/Cargo.toml
- [ ] Create `src/bin/stub_gen.rs`
- [ ] Add `stub_info` gatherer to `src/lib.rs`
- [ ] Annotate client modules (client.rs, pubmed.rs, pmc.rs)
- [ ] Annotate query builder (query.rs)
- [ ] Generate and compare stubs with manual version
- [ ] Identify methods needing manual overrides
- [ ] Add manual overrides for complex types
- [ ] Update documentation in CLAUDE.md
- [ ] Add CI check for stub freshness
- [ ] Remove or archive old manual stub file

---

## Alternative: Keep Manual with Improvements

If we decide automation overhead isn't worth it, improve manual process:

### 1. Add Stub Update Checklist to CLAUDE.md

```markdown
#### When Adding New Python Methods/Classes

1. Add Rust implementation with `#[pyclass]`/`#[pymethods]`
2. Register in `#[pymodule]` function
3. **Update `pubmed_client.pyi`** with:
   - Type signature matching Rust implementation
   - Python-friendly type hints (e.g., `list[T]` not `Vec<T>`)
   - Add to `__all__` list if new class
4. Run type check: `uv run mypy tests/ --strict`
5. Build and test: `uv run --with maturin maturin develop && uv run pytest`
```

### 2. Create Verification Script

```python
# pubmed-client-py/scripts/verify_stubs.py
"""Verify stub file completeness"""
import ast
import re
from pathlib import Path

def get_pyclass_names(rust_file: Path) -> set[str]:
    """Extract #[pyclass] names from Rust source"""
    content = rust_file.read_text()
    pattern = r'#\[pyclass(?:\([^)]*name\s*=\s*"([^"]+)"[^)]*\))?\]\s*(?:pub\s+)?struct\s+(\w+)'
    matches = re.findall(pattern, content)
    return {name or default for name, default in matches}

def get_stub_classes(pyi_file: Path) -> set[str]:
    """Extract class names from stub file"""
    tree = ast.parse(pyi_file.read_text())
    return {node.name for node in ast.walk(tree) if isinstance(node, ast.ClassDef)}

# Compare and report differences
# (Implementation details omitted for brevity)
```

### 3. Add Pre-commit Hook

```bash
# .git/hooks/pre-commit
#!/bin/bash
cd pubmed-client-py
python scripts/verify_stubs.py || {
    echo "❌ Stub file verification failed!"
    echo "Run: python scripts/verify_stubs.py for details"
    exit 1
}
```

---

## Conclusion

For the pubmed-client-py project, I recommend:

1. **Short term** (this week): Try mypy stubgen to see automated output quality
2. **Medium term** (next sprint): Pilot pyo3-stub-gen on SearchQuery class
3. **Long term** (if pilot succeeds): Full adoption with CI integration

The key decision point: Does pyo3-stub-gen reduce maintenance burden enough to justify the initial annotation investment? Given our 23 classes and ongoing development, the answer is likely **yes**, but a pilot will confirm.

---

## Resources

- [pyo3-stub-gen GitHub](https://github.com/Jij-Inc/pyo3-stub-gen)
- [pyo3-stub-gen crates.io](https://crates.io/crates/pyo3-stub-gen)
- [PyO3 Type Stub Documentation](https://pyo3.rs/main/python-typing-hints.html)
- [PEP 561 - Distributing Type Information](https://peps.python.org/pep-0561/)
- [mypy stubgen Documentation](https://mypy.readthedocs.io/en/stable/stubgen.html)
- [Maturin Issue #1942 - Auto stub generation](https://github.com/PyO3/maturin/issues/1942)

---

**Next Steps**: Review this research with the team and decide on pilot approach.
