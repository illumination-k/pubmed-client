# PyO3 0.26 Upgrade + Stub Automation Infrastructure

**Status**: Infrastructure Ready, Partial Implementation
**Date**: 2025-11-14

## What Was Done

### 1. PyO3 Upgrade ✅

- **From**: PyO3 0.23
- **To**: PyO3 0.26
- **Reason**: Required for pyo3-stub-gen 0.17 compatibility

### 2. pyo3-stub-gen Infrastructure ✅

**Added**:
- Workspace dependencies for pyo3-stub-gen 0.17
- `stub-gen` feature flag in pubmed-client-py
- `rlib` crate type for stub generation binary
- `src/bin/stub_gen.rs` binary for generating stubs

**Verified**: Successfully generates stubs when classes are annotated

### 3. Pilot Implementation ✅

**Annotated**: `ClientConfig` class in `src/config.rs`
- Added `#[cfg_attr(feature = "stub-gen", gen_stub_pyclass)]`
- Added `#[cfg_attr(feature = "stub-gen", gen_stub_pymethods)]`

## Current State

### Working

```bash
cargo run --bin stub_gen --features stub-gen
# ✓ Generated stub file: pubmed_client.pyi
```

### What's Left

**23 classes need annotation** across 11 files:

#### Data Models (14 classes)
- `src/pubmed/models.rs`: PyAffiliation, PyAuthor, PyPubMedArticle, PyRelatedArticles, PyPmcLinks, PyCitations, PyDatabaseInfo
- `src/pmc/models.rs`: PyPmcAffiliation, PyPmcAuthor, PyFigure, PyTable, PyReference, PyArticleSection, PyPmcFullText

#### Clients (3 classes)
- `src/client.rs`: PyClient
- `src/pubmed/client.rs`: PyPubMedClient
- `src/pmc/client.rs`: PyPmcClient

#### Query Builder (1 class)
- `src/query.rs`: PySearchQuery

#### Config (1 class - DONE)
- ✅ `src/config.rs`: PyClientConfig

## Path Forward

### Option A: Full Automation (Recommended)

Annotate all remaining classes to enable complete automation:

1. **Add imports** to each module:
   ```rust
   #[cfg(feature = "stub-gen")]
   use pyo3_stub_gen::derive::*;
   ```

2. **Annotate each `#[pyclass]`**:
   ```rust
   #[cfg_attr(feature = "stub-gen", gen_stub_pyclass)]
   #[pyclass(name = "ClassName")]
   pub struct PyClassName {
       // ...
   }
   ```

3. **Annotate each `#[pymethods]`**:
   ```rust
   #[cfg_attr(feature = "stub-gen", gen_stub_pymethods)]
   #[pymethods]
   impl PyClassName {
       // ...
   }
   ```

**Estimated effort**: 1-2 hours for all classes

**Result**: Fully automated stub generation with type awareness

### Option B: Hybrid Approach (Pragmatic)

Keep the semi-automated MVP for now:

1. Use `scripts/generate_stubs.sh` (runtime introspection)
2. Gradually annotate classes as they're modified
3. Transition to full automation over time

**Benefit**: Immediate value without large upfront investment

## Testing PyO3 0.26 Upgrade

### Build and Test

```bash
# Build Python package
cd pubmed-client-py
uv run --with maturin maturin develop

# Run tests
uv run pytest

# Type checking
uv run mypy tests/ --strict
```

### Known Warnings

PyO3 0.26 deprecates `Python::allow_threads()` in favor of `Python::detach()`:

```
warning: use of deprecated method `pyo3::Python::<'py>::allow_threads`: use `Python::detach` instead
```

**27 warnings total** in client code. These are non-critical but should be addressed.

**Fix**:
```rust
// Old (PyO3 0.23)
py.allow_threads(|| {
    // blocking operation
})

// New (PyO3 0.26)
py.detach(|| {
    // blocking operation
})
```

## Comparison: Full vs Semi-Automated

| Aspect | Full (pyo3-stub-gen) | Semi-Automated (MVP) |
|--------|----------------------|----------------------|
| **Type accuracy** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ |
| **Setup effort** | High (annotate 23 classes) | Low (done) |
| **Maintenance** | Very low | Low |
| **Type annotations** | Automatic | Manual |
| **Complex types** | Supports overrides | Manual only |
| **Property fields** | Automatic | Manual |
| **Build integration** | Excellent | Good |

## Recommendation

**For this upgrade session**:
1. ✅ Commit PyO3 0.26 upgrade
2. ✅ Commit automation infrastructure
3. ✅ Keep semi-automated MVP as fallback
4. ⏸️ Defer full annotation (can be separate PR)

**Next session**:
- Annotate remaining classes systematically
- Fix `allow_threads` deprecation warnings
- Remove manual stub file once automation is complete

## Migration Checklist

### Immediate (This PR)
- [x] Upgrade PyO3 to 0.26
- [x] Add pyo3-stub-gen dependencies
- [x] Create stub generation binary
- [x] Add stub_info gatherer to lib.rs
- [x] Annotate ClientConfig (pilot)
- [x] Test stub generation works
- [ ] Update CLAUDE.md with upgrade notes
- [ ] Commit and push

### Future (Separate PR)
- [ ] Annotate all data models (14 classes)
- [ ] Annotate all clients (3 classes)
- [ ] Annotate query builder (1 class)
- [ ] Fix allow_threads deprecation warnings (27 instances)
- [ ] Generate final stubs with pyo3-stub-gen
- [ ] Compare with manual stubs
- [ ] Archive or remove manual stub file
- [ ] Update documentation

## Benefits of This Approach

### Immediate
- ✅ PyO3 0.26 compatibility
- ✅ Automation infrastructure in place
- ✅ Can generate stubs for annotated classes
- ✅ Semi-automated verification still works

### Future
- 🎯 Path to full automation is clear
- 🎯 Can annotate incrementally as code is modified
- 🎯 No loss of functionality (manual stubs still work)
- 🎯 Easy to test automation with additional classes

## File Changes

### Modified
- `Cargo.toml` - Added pyo3-stub-gen workspace dependencies
- `pubmed-client-py/Cargo.toml` - Upgraded PyO3, added stub-gen feature, added rlib
- `pubmed-client-py/src/lib.rs` - Added stub_info gatherer
- `pubmed-client-py/src/config.rs` - Annotated ClientConfig

### Added
- `pubmed-client-py/src/bin/stub_gen.rs` - Stub generation binary

### Deprecated Warnings (27 instances)
- `Python::allow_threads()` → `Python::detach()` (non-critical)

## Commands

```bash
# Generate stubs (full automation - when all classes annotated)
cargo run --bin stub_gen --features stub-gen

# Generate stubs (semi-automated - runtime introspection)
./scripts/generate_stubs.sh

# Build and test
uv run --with maturin maturin develop
uv run pytest
uv run mypy tests/ --strict
```

---

**Conclusion**: Infrastructure is ready. Full automation requires annotating the remaining 22 classes, which can be done incrementally.
