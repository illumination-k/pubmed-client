Changelog
=========

All notable changes to pubmed-client-py will be documented in this file.

The format is based on `Keep a Changelog <https://keepachangelog.com/en/1.0.0/>`_,
and this project adheres to `Semantic Versioning <https://semver.org/spec/v2.0.0.html>`_.

[0.0.2] - 2024-11-18
--------------------

Added
~~~~~

- Comprehensive Sphinx documentation with user guide and API reference
- Figure extraction with captions from PMC articles
- Database information retrieval via EInfo API
- Advanced query builder with boolean logic
- Article type filtering
- Publication date filtering
- Full-text availability filtering
- Rate limiting with token bucket algorithm
- API key support for increased rate limits

Changed
~~~~~~~

- Improved error handling and error messages
- Enhanced type annotations for better IDE support
- Optimized PMC XML parsing performance

Fixed
~~~~~

- Citation count accuracy documentation
- Memory usage in large batch operations
- Rate limiting edge cases

[0.0.1] - 2024-11-01
--------------------

Initial release

Added
~~~~~

- Core PubMed client for article metadata retrieval
- PMC client for full-text article access
- Search functionality with PubMed E-utilities
- Article metadata parsing (title, authors, abstract, etc.)
- Full-text article parsing from PMC
- Markdown conversion for PMC articles
- ELink API support for related articles, citations, and PMC links
- Author and affiliation extraction
- Reference extraction
- Figure and table metadata extraction
- Type stub generation for IDE support
- Comprehensive test suite with pytest
- Python 3.12+ support
