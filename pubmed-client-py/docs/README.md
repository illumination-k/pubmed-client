# pubmed-client-py Documentation

This directory contains the Sphinx documentation for pubmed-client-py.

## Building the Documentation

### Prerequisites

Install the documentation dependencies:

```bash
cd pubmed-client-py
uv sync --group dev
```

### Build HTML Documentation

```bash
cd docs
make html
```

The built documentation will be in `_build/html/`. Open `_build/html/index.html` in your browser.

### Other Build Targets

```bash
make clean      # Remove build artifacts
make html       # Build HTML documentation
make dirhtml    # Build HTML with one directory per document
make singlehtml # Build single HTML file
make latexpdf   # Build PDF (requires LaTeX)
make epub       # Build EPUB
```

## Documentation Structure

- `index.rst` - Main documentation page
- `installation.rst` - Installation instructions
- `quickstart.rst` - Quick start guide
- `examples.rst` - Comprehensive examples
- `api/` - API reference documentation
  - `client.rst` - Client and configuration API
  - `pubmed.rst` - PubMed client and models
  - `pmc.rst` - PMC client and models
  - `query.rst` - Query builder API
  - `models.rst` - Data models overview
- `changelog.rst` - Version history
- `contributing.rst` - Contributing guidelines

## Configuration

The documentation is configured in `conf.py`. Key settings:

- **Theme**: Read the Docs theme (`sphinx_rtd_theme`)
- **Extensions**: autodoc, napoleon, viewcode, intersphinx, myst_parser
- **Intersphinx**: Links to Python documentation
- **MyST Parser**: Markdown support with enhanced features

## Live Preview

For live preview during development, you can use sphinx-autobuild:

```bash
pip install sphinx-autobuild
sphinx-autobuild docs docs/_build/html
```

Then open http://127.0.0.1:8000 in your browser.
