# Maturin Best Practices for PyO3 Development

## Known Issue: Maturin Develop Caching Problem

**⚠️ CRITICAL**: `maturin develop` has a known issue where new PyO3 methods may not be properly exported to Python, even after clean rebuilds.

**Issue Reference**: [PyO3/maturin#381](https://github.com/PyO3/maturin/issues/381)

### Symptoms

- New `#[pymethods]` functions compile without errors
- Methods don't appear in Python's `dir(object)`
- `hasattr()` returns `False` for new methods
- `cargo clean` and rebuild doesn't fix the issue

### Solution

Use `maturin build` + `pip install` instead of `maturin develop`:

```bash
# From the Python package directory
cargo clean -p <package-name>
uv run --with maturin --with patchelf maturin build --release
uv pip install target/wheels/<package-name>-*.whl --force-reinstall
```

### When to Use This Approach

- Adding new methods to existing `#[pymethods]` blocks
- Methods appear in source code but not in Python
- After extensive debugging with `maturin develop`

### When `maturin develop` is Safe

- Making changes to existing method implementations
- Adding entirely new classes (not methods to existing classes)
- Working on Rust-only changes

## Module Registration

### Rule 1: Always Add New #[pyclass] Types to #[pymodule]

```rust
#[pymodule]
fn your_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Add ALL pyclass types here
    m.add_class::<YourNewClass>()?;
    m.add_class::<AnotherClass>()?;
    Ok(())
}
```

### Rule 2: Keep #[pyclass] and #[pymethods] in the Same File

PyO3 requires both to be in the same Rust module. Splitting them across files will cause silent export failures.

### Verification Steps

After adding new classes:

```bash
# Rebuild with maturin
uv run --with maturin maturin develop

# Test that class is accessible
uv run python -c "from your_module import YourNewClass; print('Success!')"

# Or inspect the module
uv run python -c "import your_module; print('YourNewClass' in dir(your_module))"
```

## Build and Testing Workflow

### Development Builds

```bash
# Development build (faster, with debug symbols)
uv run --with maturin maturin develop

# Release build (optimized)
uv run --with maturin maturin develop --release
```

### Clean Builds for Troubleshooting

```bash
# Clean Rust artifacts
cargo clean -p your-package-name

# Rebuild
uv run --with maturin maturin develop
```

### Test Immediately After Changes

```bash
# Run specific test file
uv run pytest tests/test_your_feature.py -v

# Run all tests
uv run pytest

# Run with coverage
uv run pytest --cov=your_module --cov-report=html
```

## Type Stubs (.pyi files) - Automatic Generation

**IMPORTANT**: Type stubs should be automatically generated using `pyo3-stub-gen`. DO NOT manually edit `.pyi` files.

### Stub Generation Workflow

1. **Add pyo3-stub-gen macros to PyO3 code:**

```rust
use pyo3_stub_gen_derive::{gen_stub_pyclass, gen_stub_pymethods};

#[gen_stub_pyclass]
#[pyclass(name = "YourClass")]
pub struct PyYourClass {
    // fields
}

#[gen_stub_pymethods]
#[pymethods]
impl PyYourClass {
    // methods
}
```

2. **Generate stubs after making changes:**

```bash
# From the Python package directory
cargo run --bin stub_gen

# Copy generated stub to the correct location
cp your_package/your_module.pyi your_module.pyi
```

3. **Test type checking:**

```bash
uv run mypy tests/ --strict
```

### Handling Complex Types

For types that don't automatically work with pyo3-stub-gen (like `FromPyObject` enums), implement `PyStubType`:

```rust
use pyo3_stub_gen::PyStubType;

enum QueryInput {
    String(String),
    SearchQuery(PySearchQuery),
}

impl PyStubType for QueryInput {
    fn type_output() -> pyo3_stub_gen::TypeInfo {
        pyo3_stub_gen::TypeInfo::builtin("str | SearchQuery")
    }
}
```

## Common Pitfalls

### 1. Module Name Confusion

The `module-name` in `pyproject.toml` creates a nested structure:

- For `module-name = "your_package.your_module"`:
  - The .so file is `your_module.cpython-312-*.so`
  - Python imports via `from your_module import Class`

### 2. Type Mismatches

- Python int → Rust usize: Negative values cause `OverflowError`
- Use appropriate error handling in tests
- PyO3 type validation happens before your Rust code runs

### 3. Silent Export Failures

If a class compiles but doesn't export, check:

- Is it in `m.add_class::<YourClass>()?`?
- Are #[pyclass] and #[pymethods] in the same file?
- Did you rebuild with `maturin develop`?

### 4. Cache Issues

- Python caches imported modules
- Restart Python interpreter after rebuilding
- Or use `importlib.reload()` for iterative development

## Debugging Import Issues

If a class doesn't appear in Python after adding it:

```bash
# 1. Check if it's in the compiled .so module
uv run python << 'EOF'
import your_package.your_module as so
print("YourClass" in dir(so))
print(so.__all__ if hasattr(so, "__all__") else "No __all__")
EOF

# 2. Check if it's exported from the package
uv run python -c "import your_package; print('YourClass' in dir(your_package))"

# 3. Try direct import
uv run python -c "from your_package import YourClass; print(YourClass)"
```

### If the Class is in .so But Not in Package

- Check `__all__` list in type stubs
- Verify `__init__.py` has `from .your_module import *`
- Try uninstalling and reinstalling:
  ```bash
  uv pip uninstall your-package
  uv run --with maturin maturin develop
  ```

## Rate of Success Indicators

### Signs Everything is Working

✅ All #[pyclass] types appear in `dir(module)`
✅ `hasattr(instance, 'method_name')` returns `True` for all methods
✅ Direct imports work: `from module import ClassName`
✅ Type stubs match runtime behavior
✅ `mypy` type checking passes
✅ All pytest tests pass

### Signs Something is Wrong

❌ Methods compile but don't appear in Python
❌ Import errors despite successful compilation
❌ Type stub mismatches with runtime
❌ Cached old behavior after code changes
❌ Silent failures with no error messages
