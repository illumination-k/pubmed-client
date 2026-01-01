Contributing
============

Thank you for your interest in contributing to pubmed-client-py!

Development Setup
-----------------

Prerequisites
~~~~~~~~~~~~~

- Python 3.12 or later
- Rust toolchain (https://rustup.rs/)
- uv package manager (recommended)

Installation
~~~~~~~~~~~~

1. Clone the repository:

.. code-block:: bash

   git clone https://github.com/illumination-k/pubmed-client-rs.git
   cd pubmed-client-rs/pubmed-client-py

2. Install development dependencies:

.. code-block:: bash

   uv sync --group dev

3. Build the extension:

.. code-block:: bash

   uv run --with maturin maturin develop

Running Tests
-------------

Run the full test suite:

.. code-block:: bash

   uv run pytest

Run with coverage:

.. code-block:: bash

   uv run pytest --cov=pubmed_client --cov-report=html

Run specific test files:

.. code-block:: bash

   uv run pytest tests/test_client.py
   uv run pytest tests/test_integration.py

Run tests by marker:

.. code-block:: bash

   # Unit tests only
   uv run pytest -m "not integration"

   # Integration tests only (requires network)
   uv run pytest -m integration

Code Quality
------------

Type Checking
~~~~~~~~~~~~~

Run mypy for type checking:

.. code-block:: bash

   uv run mypy tests/ --strict

Linting and Formatting
~~~~~~~~~~~~~~~~~~~~~~

Run ruff for linting:

.. code-block:: bash

   uv run ruff check .

Format code:

.. code-block:: bash

   uv run ruff format .

Building Documentation
----------------------

Build the Sphinx documentation:

.. code-block:: bash

   cd docs
   make html

Open the built documentation:

.. code-block:: bash

   open _build/html/index.html  # macOS
   xdg-open _build/html/index.html  # Linux

Clean build artifacts:

.. code-block:: bash

   make clean

Making Changes
--------------

Workflow
~~~~~~~~

1. Create a new branch for your changes:

.. code-block:: bash

   git checkout -b feature/your-feature-name

2. Make your changes with proper tests
3. Run the test suite and ensure all tests pass
4. Run type checking and linting
5. Update documentation if needed
6. Commit your changes with descriptive messages
7. Push to your fork and create a pull request

Commit Messages
~~~~~~~~~~~~~~~

Use clear, descriptive commit messages:

- Use the imperative mood ("Add feature" not "Added feature")
- Start with a capital letter
- Keep the first line under 72 characters
- Reference issues and PRs where appropriate

Examples:

.. code-block:: text

   Add figure extraction with captions

   - Implement extract_figures_with_captions method
   - Add ExtractedFigure model with file information
   - Include dimension detection for images
   - Add comprehensive tests

   Fixes #123

Code Style Guidelines
---------------------

Python Code
~~~~~~~~~~~

- Follow PEP 8 style guide
- Use type hints for all function signatures
- Write docstrings for public APIs
- Keep functions focused and simple
- Prefer explicit over implicit

Rust Code
~~~~~~~~~

- Follow Rust standard style (rustfmt)
- Use clippy for linting
- Write comprehensive error messages
- Document public APIs with doc comments
- Use Result types for error handling

Testing Guidelines
------------------

Writing Tests
~~~~~~~~~~~~~

- Write tests for all new features
- Include both success and failure cases
- Use descriptive test names
- Keep tests isolated and independent
- Mock external API calls when appropriate

Test Organization
~~~~~~~~~~~~~~~~~

- Unit tests: Test individual functions/methods
- Integration tests: Test API interactions (marked with ``@pytest.mark.integration``)
- Use fixtures for common setup code
- Group related tests in classes

Example Test
~~~~~~~~~~~~

.. code-block:: python

   import pytest
   from pubmed_client import Client

   def test_search_articles_basic():
       """Test basic article search functionality."""
       client = Client()
       articles = client.pubmed.search_and_fetch("test query", 5)

       assert len(articles) <= 5
       assert all(a.pmid for a in articles)
       assert all(a.title for a in articles)

   @pytest.mark.integration
   def test_fetch_real_article():
       """Integration test: Fetch a known article."""
       client = Client()
       article = client.pubmed.fetch_article("31978945")

       assert article.pmid == "31978945"
       assert "COVID-19" in article.title or "coronavirus" in article.title.lower()

Documentation Guidelines
------------------------

Docstring Format
~~~~~~~~~~~~~~~~

Use Google-style docstrings:

.. code-block:: python

   def fetch_article(self, pmid: str) -> PubMedArticle:
       """Fetch a single article by PMID.

       Args:
           pmid: PubMed ID as a string

       Returns:
           PubMedArticle object

       Raises:
           ValueError: If PMID is invalid
           NetworkError: If API request fails

       Example:
           >>> client = PubMedClient()
           >>> article = client.fetch_article("31978945")
           >>> print(article.title)
       """

RST Documentation
~~~~~~~~~~~~~~~~~

- Keep documentation up to date with code changes
- Include practical examples
- Add cross-references to related APIs
- Update the changelog for significant changes

Reporting Issues
----------------

Bug Reports
~~~~~~~~~~~

When reporting bugs, include:

- Python version
- pubmed-client-py version
- Operating system
- Minimal code to reproduce the issue
- Expected vs actual behavior
- Full error traceback

Feature Requests
~~~~~~~~~~~~~~~~

When requesting features, include:

- Clear description of the feature
- Use cases and motivation
- Example API you'd like to see
- Any relevant PubMed/PMC API documentation

Pull Request Process
---------------------

1. Ensure all tests pass and coverage is maintained
2. Update documentation for any API changes
3. Add entries to CHANGELOG.md under "Unreleased"
4. Request review from maintainers
5. Address any feedback
6. Squash commits if requested

Review Criteria
~~~~~~~~~~~~~~~

Pull requests will be reviewed for:

- Code quality and style
- Test coverage
- Documentation completeness
- Performance implications
- Breaking changes
- Security considerations

Release Process
---------------

Releases are managed by maintainers following semantic versioning:

- **Major** version: Breaking changes
- **Minor** version: New features (backward compatible)
- **Patch** version: Bug fixes (backward compatible)

Getting Help
------------

- GitHub Issues: https://github.com/illumination-k/pubmed-client-rs/issues
- Discussions: https://github.com/illumination-k/pubmed-client-rs/discussions

License
-------

By contributing to pubmed-client-py, you agree that your contributions will be licensed under the MIT License.
