Quickstart
==========

This guide will get you started with the basic features of pubmed-client-py.

Basic Client Usage
------------------

Creating a Client
~~~~~~~~~~~~~~~~~

The simplest way to get started:

.. code-block:: python

    import pubmed_client

    # Create a client with default configuration
    client = pubmed_client.Client()

With custom configuration:

.. code-block:: python

    config = pubmed_client.ClientConfig()\\
        .with_api_key("your_api_key")\\
        .with_email("you@example.com")\\
        .with_tool("MyResearchTool")\\
        .with_rate_limit(10.0)  # 10 requests per second

    client = pubmed_client.Client.with_config(config)

Searching PubMed
----------------

Simple Search
~~~~~~~~~~~~~

.. code-block:: python

    # Search and fetch article metadata
    articles = client.pubmed.search_and_fetch("covid-19", 10)

    for article in articles:
        print(f"Title: {article.title}")
        print(f"PMID: {article.pmid}")
        print(f"Journal: {article.journal}")
        print(f"Authors: {len(article.authors())} authors")
        print()

Search with Field Tags
~~~~~~~~~~~~~~~~~~~~~~

PubMed supports field-specific searches using tags:

.. code-block:: python

    # Search in title only
    articles = client.pubmed.search_and_fetch("covid-19[ti]", 10)

    # Search in title/abstract
    articles = client.pubmed.search_and_fetch("vaccine[tiab]", 10)

    # Search by author
    articles = client.pubmed.search_and_fetch("Smith J[au]", 10)

    # Search by journal
    articles = client.pubmed.search_and_fetch("Nature[ta]", 10)

Advanced Query Builder
~~~~~~~~~~~~~~~~~~~~~~

Use the SearchQuery builder for complex queries:

.. code-block:: python

    from pubmed_client import SearchQuery

    # Build a complex query
    query = SearchQuery()\\
        .query("covid-19")\\
        .published_between(2020, 2024)\\
        .article_types(["Clinical Trial", "Meta-Analysis"])\\
        .free_full_text_only()\\
        .limit(50)

    articles = client.pubmed.search_and_fetch(query, 0)

    # Boolean operations
    q1 = SearchQuery().query("cancer")
    q2 = SearchQuery().query("treatment")
    combined = q1.and_(q2)

    articles = client.pubmed.search_and_fetch(combined, 20)

Fetching PMC Full-Text
----------------------

Checking PMC Availability
~~~~~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: python

    # Check if PMC full-text is available for a PMID
    pmcid = client.pmc.check_pmc_availability("31978945")
    if pmcid:
        print(f"PMC full-text available: {pmcid}")

Fetching Full-Text
~~~~~~~~~~~~~~~~~~

.. code-block:: python

    # Fetch full-text article
    full_text = client.pmc.fetch_full_text("PMC7906746")

    print(f"Title: {full_text.title}")
    print(f"PMCID: {full_text.pmcid}")
    print(f"DOI: {full_text.doi}")

    # Access sections
    for section in full_text.sections():
        print(f"Section: {section.title}")
        print(section.content[:200])  # First 200 characters

    # Access figures
    for figure in full_text.figures():
        print(f"{figure.label}: {figure.caption}")

    # Access references
    for ref in full_text.references():
        print(f"{ref.title} ({ref.year})")

Converting to Markdown
~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: python

    full_text = client.pmc.fetch_full_text("PMC7906746")
    markdown = full_text.to_markdown()

    # Save to file
    with open("article.md", "w") as f:
        f.write(markdown)

Extracting Figures
------------------

Basic Figure Extraction
~~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: python

    # Extract all figures with captions
    figures = client.pmc.extract_figures_with_captions("PMC7906746", "./output")

    for fig in figures:
        print(f"Figure: {fig.figure.label}")
        print(f"Caption: {fig.figure.caption}")
        print(f"File: {fig.extracted_file_path}")
        print(f"Size: {fig.file_size} bytes")
        if fig.dimensions:
            width, height = fig.dimensions
            print(f"Dimensions: {width}x{height}")
        print()

Working with Article Relationships
-----------------------------------

Finding Related Articles
~~~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: python

    # Find related articles (ELink API)
    related = client.get_related_articles([31978945, 33515491])

    print(f"Found {len(related)} related articles")
    for pmid in related.related_pmids:
        print(f"PMID: {pmid}")

Checking PMC Links
~~~~~~~~~~~~~~~~~~

.. code-block:: python

    # Check PMC availability for multiple PMIDs
    pmc_links = client.get_pmc_links([31978945, 33515491])

    for pmcid in pmc_links.pmc_ids:
        print(f"PMC ID: {pmcid}")

Finding Citations
~~~~~~~~~~~~~~~~~

.. code-block:: python

    # Find articles that cite the given PMIDs
    citations = client.get_citations([31978945])

    print(f"Found {len(citations)} citing articles")
    for pmid in citations.citing_pmids[:10]:  # First 10
        print(f"Citing PMID: {pmid}")

Database Information
--------------------

Listing Databases
~~~~~~~~~~~~~~~~~

.. code-block:: python

    # Get list of all NCBI databases
    databases = client.get_database_list()

    for db in databases:
        print(db)

Getting Database Details
~~~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: python

    # Get detailed information about a database
    info = client.get_database_info("pubmed")

    print(f"Name: {info.name}")
    print(f"Description: {info.description}")
    print(f"Record Count: {info.count}")
    print(f"Last Updated: {info.last_update}")

Combined Search with Full-Text
-------------------------------

.. code-block:: python

    # Search and attempt to fetch full-text for each result
    results = client.search_with_full_text("covid-19", 5)

    for article, full_text in results:
        print(f"Title: {article.title}")
        if full_text:
            print(f"  Has full-text: {full_text.pmcid}")
            print(f"  Sections: {len(full_text.sections())}")
        else:
            print("  No full-text available")

Error Handling
--------------

.. code-block:: python

    try:
        articles = client.pubmed.search_and_fetch("invalid query", 10)
    except Exception as e:
        print(f"Search failed: {e}")

    try:
        full_text = client.pmc.fetch_full_text("PMC999999999")
    except Exception as e:
        print(f"PMC fetch failed: {e}")

Next Steps
----------

- Read the :doc:`examples` for more detailed use cases
- Explore the :doc:`api/client` for complete API documentation
- Check the GitHub repository for source code and examples
