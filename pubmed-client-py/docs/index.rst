Welcome to pubmed-client-py's documentation!
============================================

**pubmed-client-py** is a high-performance Python library for accessing PubMed and PMC (PubMed Central) APIs. Built with Rust and PyO3, it provides a fast, type-safe interface for retrieving biomedical research articles.

Features
--------

- 🚀 **High Performance**: Built with Rust for maximum speed
- 🔍 **Comprehensive API**: Full support for PubMed and PMC E-utilities
- 📚 **Rich Metadata**: Access article metadata, abstracts, full-text, and more
- 🔗 **Article Relationships**: Find related articles, citations, and PMC links via ELink API
- 🗄️ **Database Information**: Query NCBI database details via EInfo API
- 🎨 **Figure Extraction**: Download and extract figures with captions from PMC articles
- 🔎 **Advanced Queries**: Builder pattern for constructing complex search queries
- 📝 **Markdown Conversion**: Convert PMC articles to Markdown format
- ⚡ **Rate Limiting**: Built-in rate limiting compliant with NCBI guidelines
- 🔑 **API Key Support**: Optional API key for increased rate limits (10 req/s vs 3 req/s)
- 🛡️ **Type Safety**: Complete type hints with mypy support

Quick Example
-------------

.. code-block:: python

    import pubmed_client

    # Create a client
    client = pubmed_client.Client()

    # Search PubMed
    articles = client.pubmed.search_and_fetch("covid-19", 10)
    for article in articles:
        print(f"{article.title} (PMID: {article.pmid})")

    # Fetch PMC full-text
    full_text = client.pmc.fetch_full_text("PMC7906746")
    print(full_text.title)

    # Convert to Markdown
    markdown = full_text.to_markdown()
    print(markdown)

    # Extract figures with captions
    figures = client.pmc.extract_figures_with_captions("PMC7906746", "./output")
    for fig in figures:
        print(f"{fig.figure.label}: {fig.extracted_file_path}")

Contents
--------

.. toctree::
   :maxdepth: 2
   :caption: User Guide

   installation
   quickstart
   examples

.. toctree::
   :maxdepth: 2
   :caption: API Reference

   api/client
   api/pubmed
   api/pmc
   api/query
   api/models

.. toctree::
   :maxdepth: 1
   :caption: Additional Information

   changelog
   contributing

Indices and tables
==================

* :ref:`genindex`
* :ref:`modindex`
* :ref:`search`
