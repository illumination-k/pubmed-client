# Stub Generation MVP - Implementation Guide

**Status**: MVP Implemented (Runtime Inspection Approach)
**Date**: 2025-11-14
**See also**: [STUB_GENERATION_RESEARCH.md](STUB_GENERATION_RESEARCH.md)

## Summary

This MVP provides a **semi-automated approach** to stub generation using runtime introspection. While not fully automated, it significantly reduces the risk of missing classes/methods in manual stub files.

## What Works

✅ **Automated extraction of**:

- All classes in the module
- All public methods with signatures
- Method names and basic parameter info
- Docstrings from Rust source
- Proper `__all__` list generation

## What's Missing (Requires Manual Work)

❌ **Not automatically extracted**:

- Type annotations (parameter and return types)
- Property fields (e.g., `pmid: str`, `title: str`)
- Complex types (`Union[str, SearchQuery]`, `list[T] | None`)
- Optional parameter defaults

## The MVP Solution

### Components

1. **`scripts/inspect_module.py`** - Python script that uses runtime introspection
2. **`scripts/generate_stubs.sh`** - Wrapper script for easy execution
3. **`pubmed_client_auto.pyi`** - Generated baseline stub file (192 lines)
4. **`pubmed_client.pyi`** - Manual stub file with full type annotations (223 lines)

### Quick Start

```bash
# From pubmed-client-py/
./scripts/generate_stubs.sh
```

This will:

1. Build the package with maturin
2. Generate `pubmed_client_auto.pyi` using runtime introspection
3. Show line count comparison
4. Suggest next steps

### Sample Output

```bash
🔨 Building Python package with maturin...
📝 Generating stubs using runtime introspection...
✅ Generated: pubmed_client_auto.pyi

📊 Comparison:
  Manual stub:      223 lines
  Auto-generated:   192 lines

💡 Next steps:
  1. Review: diff pubmed_client.pyi pubmed_client_auto.pyi
  2. Check for missing classes/methods in manual stub
  3. Add type annotations to auto-generated stubs if needed
```

## Use Cases

### 1. Verify Manual Stub Completeness

After adding new Rust methods:

```bash
./scripts/generate_stubs.sh
diff pubmed_client.pyi pubmed_client_auto.pyi
```

This reveals any classes/methods missing from the manual stub.

### 2. Bootstrap New Module Stubs

For a new module:

```bash
./scripts/generate_stubs.sh
# Copy pubmed_client_auto.pyi as starting point
# Add type annotations manually
```

### 3. CI Verification

Add to GitHub Actions to ensure manual stubs are complete:

```yaml
- name: Verify stub completeness
  run: |
    cd pubmed-client-py
    ./scripts/generate_stubs.sh
    # Compare method counts or class lists
```

## Comparison: Auto vs Manual

### Auto-Generated (`pubmed_client_auto.pyi`)

```python
class ClientConfig:
    """Python wrapper for ClientConfig"""
    def __init__(self, /, *args, **kwargs): ...
    def __repr__(self, /): ...
    def __str__(self, /): ...
    def with_api_key(self, /, api_key): ...
    def with_cache(self, /): ...
    def with_email(self, /, email): ...
    def with_rate_limit(self, /, rate_limit): ...
    def with_timeout_seconds(self, /, timeout_seconds): ...
    def with_tool(self, /, tool): ...
```

### Manual with Types (`pubmed_client.pyi`)

```python
class ClientConfig:
    def __init__(self) -> None: ...
    def with_api_key(self, api_key: str) -> ClientConfig: ...
    def with_email(self, email: str) -> ClientConfig: ...
    def with_tool(self, tool: str) -> ClientConfig: ...
    def with_rate_limit(self, rate_limit: float) -> ClientConfig: ...
    def with_timeout_seconds(self, timeout_seconds: int) -> ClientConfig: ...
    def with_cache(self) -> ClientConfig: ...
```

**Key differences**:

- Manual has proper type annotations (`str`, `ClientConfig`, `int`)
- Manual has correct return type (builder pattern returns `ClientConfig`)
- Auto-generated has generic signatures with no type info

## Why Other Tools Didn't Work

### mypy stubgen

- Generated almost empty stubs
- Only detected module structure, no method details
- Not PyO3-aware enough

```bash
# Output: Only __version__ and empty class references
stubgen -p pubmed_client --include-docstrings -o stubgen_output/
```

### pyo3-stubgen (Python package)

- Generated empty stub file
- Documentation states: "Currently only generates info for functions; classes are on the to-do list"
- Not suitable for class-based APIs

```bash
# Output: Nearly empty .pyi file
pyo3-stubgen pubmed_client pyo3stubgen_output/
```

### pyo3-stub-gen (Rust crate)

- **Version conflict**: Requires PyO3 0.26, project uses PyO3 0.23
- Would require either:
  - Upgrading PyO3 (potentially breaking changes)
  - Finding compatible older version (not well documented)
  - Annotating all Rust code with macros (significant overhead)

**Error encountered**:

```
error: failed to select a version for `pyo3-ffi`.
package `pyo3-ffi` links to the native library `python`, but it conflicts...
```

## Decision: Why This MVP?

1. **No code changes required** - works with existing PyO3 0.23
2. **Immediate value** - catches missing methods/classes
3. **Low overhead** - single script, runs in seconds
4. **Path forward** - establishes workflow for future automation
5. **Practical** - solves the main pain point (forgetting to update stubs)

## Future Improvements

### Short Term (Next Week)

- [ ] Add CI check to verify stub completeness
- [ ] Document in CLAUDE.md when to run the script
- [ ] Add pre-commit hook option

### Medium Term (Next Sprint)

If PyO3 is upgraded to 0.26:

- [ ] Re-evaluate pyo3-stub-gen with compatible version
- [ ] Pilot on config module
- [ ] Measure annotation overhead vs. maintenance savings

### Long Term

- [ ] Contribute to pyo3-stubgen class support
- [ ] Or build custom tool using PyO3's introspection API
- [ ] Explore integrating with maturin build process

## Maintenance Workflow

### When Adding New Methods

1. **Add Rust implementation** with `#[pymethods]`
2. **Register in `#[pymodule]`** function
3. **Run**: `./scripts/generate_stubs.sh`
4. **Compare**: `diff pubmed_client.pyi pubmed_client_auto.pyi`
5. **Update manual stub** with type annotations
6. **Test**: `uv run mypy tests/ --strict`
7. **Verify**: `uv run --with maturin maturin develop && uv run pytest`

### Quality Checks

```bash
# Check for missing classes/methods
./scripts/generate_stubs.sh
diff -u pubmed_client.pyi pubmed_client_auto.pyi | grep "^+    def" | head -20

# Verify types
uv run mypy tests/ --strict

# Test import
uv run python -c "from pubmed_client import *; print('OK')"
```

## Recommendations

### For Daily Development

✅ **Use this MVP** for verification and bootstrapping

- Quick to run
- No build complexity
- Catches common mistakes

### For Production

✅ **Keep manual stubs** for now

- Full type annotations
- IDE autocomplete works perfectly
- mypy validation passes

### For Future

⏳ **Upgrade to PyO3 0.26** when convenient

- Then re-evaluate pyo3-stub-gen
- Potential for full automation
- But MVP remains useful as fallback

## Conclusion

This MVP provides **80% of the value** with **20% of the complexity**:

- ✅ Automated detection of classes/methods
- ✅ No code changes required
- ✅ Works with current PyO3 version
- ✅ Simple script anyone can run
- ❌ Still requires manual type annotations (acceptable trade-off)

**Bottom line**: This is a practical solution that solves the main pain point (missing classes/methods) without the complexity of full automation. It's production-ready and can be used immediately.

---

## Related Files

- [STUB_GENERATION_RESEARCH.md](STUB_GENERATION_RESEARCH.md) - Full research on all approaches
- [scripts/inspect_module.py](scripts/inspect_module.py) - Introspection script
- [scripts/generate_stubs.sh](scripts/generate_stubs.sh) - Wrapper script
- [pubmed_client.pyi](pubmed_client.pyi) - Manual stubs (current)
- `pubmed_client_auto.pyi` - Auto-generated stubs (generated)
