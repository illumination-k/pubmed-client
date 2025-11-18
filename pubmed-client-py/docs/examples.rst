Examples
========

This page contains practical examples for common use cases.

Example 1: COVID-19 Research Analysis
--------------------------------------

Search for COVID-19 vaccine research and analyze the results:

.. code-block:: python

    import pubmed_client
    from pubmed_client import SearchQuery

    # Create client with API key for better rate limits
    config = pubmed_client.ClientConfig()\\
        .with_api_key("your_api_key")\\
        .with_email("you@example.com")

    client = pubmed_client.Client.with_config(config)

    # Build a targeted query
    query = SearchQuery()\\
        .query("covid-19")\\
        .query("vaccine")\\
        .published_between(2020, 2024)\\
        .article_types(["Clinical Trial", "Meta-Analysis"])\\
        .free_full_text_only()\\
        .limit(100)

    # Fetch articles
    articles = client.pubmed.search_and_fetch(query, 0)

    # Analyze results
    print(f"Found {len(articles)} articles")

    # Group by year
    by_year = {}
    for article in articles:
        year = article.pub_date[:4]  # Extract year
        by_year[year] = by_year.get(year, 0) + 1

    print("\\nArticles by year:")
    for year in sorted(by_year.keys()):
        print(f"  {year}: {by_year[year]}")

    # Extract author information
    all_authors = []
    for article in articles:
        all_authors.extend([a.full_name for a in article.authors()])

    print(f"\\nTotal authors: {len(all_authors)}")
    print(f"Unique authors: {len(set(all_authors))}")

Example 2: Downloading Full-Text Articles
------------------------------------------

Batch download PMC full-text articles:

.. code-block:: python

    import pubmed_client
    import os

    client = pubmed_client.Client()

    # PMC IDs to download
    pmc_ids = ["PMC7906746", "PMC8240743", "PMC8359465"]

    output_dir = "./pmc_articles"
    os.makedirs(output_dir, exist_ok=True)

    for pmc_id in pmc_ids:
        try:
            # Fetch full-text
            full_text = client.pmc.fetch_full_text(pmc_id)

            # Convert to Markdown
            markdown = full_text.to_markdown()

            # Save to file
            filename = f"{output_dir}/{pmc_id}.md"
            with open(filename, "w", encoding="utf-8") as f:
                f.write(markdown)

            print(f"Downloaded: {pmc_id} -> {filename}")

        except Exception as e:
            print(f"Failed to download {pmc_id}: {e}")

Example 3: Figure Extraction Pipeline
--------------------------------------

Extract figures from multiple articles:

.. code-block:: python

    import pubmed_client
    import os
    import json

    client = pubmed_client.Client()

    pmc_ids = ["PMC7906746", "PMC8240743"]
    base_output_dir = "./figures"

    results = []

    for pmc_id in pmc_ids:
        try:
            # Create output directory for this article
            output_dir = f"{base_output_dir}/{pmc_id}"
            os.makedirs(output_dir, exist_ok=True)

            # Extract figures with captions
            figures = client.pmc.extract_figures_with_captions(pmc_id, output_dir)

            # Collect metadata
            article_data = {
                "pmc_id": pmc_id,
                "figure_count": len(figures),
                "figures": []
            }

            for fig in figures:
                fig_data = {
                    "id": fig.figure.id,
                    "label": fig.figure.label,
                    "caption": fig.figure.caption,
                    "file_path": fig.extracted_file_path,
                    "file_size": fig.file_size,
                    "dimensions": fig.dimensions
                }
                article_data["figures"].append(fig_data)

            results.append(article_data)
            print(f"Extracted {len(figures)} figures from {pmc_id}")

        except Exception as e:
            print(f"Failed to process {pmc_id}: {e}")

    # Save metadata to JSON
    with open(f"{base_output_dir}/metadata.json", "w") as f:
        json.dump(results, f, indent=2)

Example 4: Citation Network Analysis
-------------------------------------

Build a citation network for a set of articles:

.. code-block:: python

    import pubmed_client

    client = pubmed_client.Client()

    # Starting PMIDs
    seed_pmids = [31978945, 33515491]

    # Get citations (articles that cite these)
    citations = client.get_citations(seed_pmids)

    print(f"Found {len(citations)} citations")
    print(f"Citing PMIDs: {citations.citing_pmids[:10]}")  # First 10

    # Get related articles
    related = client.get_related_articles(seed_pmids)

    print(f"\\nFound {len(related)} related articles")
    print(f"Related PMIDs: {related.related_pmids[:10]}")  # First 10

    # Check which related articles have PMC full-text
    if related.related_pmids:
        pmc_links = client.get_pmc_links(related.related_pmids[:50])  # First 50
        print(f"\\nRelated articles with PMC full-text: {len(pmc_links)}")

Example 5: Journal Analysis
----------------------------

Analyze publication trends in a specific journal:

.. code-block:: python

    import pubmed_client
    from pubmed_client import SearchQuery
    from collections import Counter

    client = pubmed_client.Client()

    # Search Nature journal for cancer research
    query = SearchQuery()\\
        .query("Nature[ta]")\\
        .query("cancer[tiab]")\\
        .published_between(2020, 2024)\\
        .limit(500)

    articles = client.pubmed.search_and_fetch(query, 0)

    print(f"Found {len(articles)} articles")

    # Analyze article types
    article_types = []
    for article in articles:
        article_types.extend(article.article_types())

    type_counts = Counter(article_types)
    print("\\nArticle types:")
    for article_type, count in type_counts.most_common(10):
        print(f"  {article_type}: {count}")

    # Analyze publication dates
    pub_years = [article.pub_date[:4] for article in articles]
    year_counts = Counter(pub_years)

    print("\\nPublications by year:")
    for year in sorted(year_counts.keys()):
        print(f"  {year}: {year_counts[year]}")

Example 6: Author Collaboration Analysis
-----------------------------------------

Identify prolific authors and their collaborations:

.. code-block:: python

    import pubmed_client
    from pubmed_client import SearchQuery
    from collections import Counter
    from itertools import combinations

    client = pubmed_client.Client()

    # Search for CRISPR research
    query = SearchQuery()\\
        .query("CRISPR")\\
        .published_between(2020, 2024)\\
        .article_types(["Research Article"])\\
        .limit(200)

    articles = client.pubmed.search_and_fetch(query, 0)

    # Count author appearances
    author_counts = Counter()
    collaborations = Counter()

    for article in articles:
        authors = [a.full_name for a in article.authors()]
        author_counts.update(authors)

        # Track collaborations (pairs of authors)
        for pair in combinations(sorted(authors), 2):
            collaborations[pair] += 1

    # Top authors
    print("Top 10 authors:")
    for author, count in author_counts.most_common(10):
        print(f"  {author}: {count} papers")

    # Top collaborations
    print("\\nTop 10 collaborations:")
    for (author1, author2), count in collaborations.most_common(10):
        print(f"  {author1} & {author2}: {count} papers")

Example 7: Database Exploration
--------------------------------

Explore NCBI databases and their properties:

.. code-block:: python

    import pubmed_client

    client = pubmed_client.Client()

    # Get list of all databases
    databases = client.get_database_list()
    print(f"Available databases: {len(databases)}")

    # Get details for specific databases
    interesting_dbs = ["pubmed", "pmc", "gene", "protein"]

    for db_name in interesting_dbs:
        try:
            info = client.get_database_info(db_name)
            print(f"\\n{info.name}:")
            print(f"  Menu Name: {info.menu_name}")
            print(f"  Description: {info.description[:100]}...")
            print(f"  Records: {info.count:,}")
            print(f"  Last Updated: {info.last_update}")
        except Exception as e:
            print(f"  Failed: {e}")

Example 8: Advanced Boolean Queries
------------------------------------

Construct complex queries using boolean logic:

.. code-block:: python

    import pubmed_client
    from pubmed_client import SearchQuery

    client = pubmed_client.Client()

    # Build complex query: (cancer OR tumor) AND (treatment OR therapy) NOT review
    cancer_query = SearchQuery().query("cancer").or_(SearchQuery().query("tumor"))
    treatment_query = SearchQuery().query("treatment").or_(SearchQuery().query("therapy"))
    review_query = SearchQuery().query("review[pt]")

    final_query = cancer_query\\
        .and_(treatment_query)\\
        .exclude(review_query)\\
        .published_between(2022, 2024)\\
        .free_full_text_only()\\
        .limit(100)

    # Execute search
    articles = client.pubmed.search_and_fetch(final_query, 0)

    print(f"Query: {final_query.build()}")
    print(f"Found {len(articles)} articles")

    # Display results
    for article in articles[:5]:
        print(f"\\n{article.title}")
        print(f"  PMID: {article.pmid}")
        print(f"  Journal: {article.journal}")
        print(f"  Date: {article.pub_date}")

Example 9: Batch Processing with Rate Limiting
-----------------------------------------------

Process large batches efficiently:

.. code-block:: python

    import pubmed_client
    import time

    # Configure with appropriate rate limit
    config = pubmed_client.ClientConfig()\\
        .with_api_key("your_api_key")\\  # 10 req/s with key
        .with_email("you@example.com")\\
        .with_rate_limit(9.0)  # Slightly under limit for safety

    client = pubmed_client.Client.with_config(config)

    # Large list of PMIDs to process
    pmids = [f"{i}" for i in range(30000000, 30001000)]  # 1000 PMIDs

    batch_size = 100
    results = []

    for i in range(0, len(pmids), batch_size):
        batch = pmids[i:i + batch_size]

        try:
            # Fetch articles in batch
            for pmid in batch:
                try:
                    article = client.pubmed.fetch_article(pmid)
                    results.append(article)
                except Exception:
                    pass  # Skip invalid PMIDs

            print(f"Processed batch {i//batch_size + 1}: {len(results)} total articles")

        except Exception as e:
            print(f"Batch {i//batch_size + 1} failed: {e}")
            time.sleep(5)  # Back off on error

    print(f"\\nTotal articles fetched: {len(results)}")

Example 10: Combined Workflow
------------------------------

Complete workflow from search to figure extraction:

.. code-block:: python

    import pubmed_client
    from pubmed_client import SearchQuery
    import os

    # Initialize client
    client = pubmed_client.Client()

    # Step 1: Search for articles
    query = SearchQuery()\\
        .query("deep learning")\\
        .query("medical imaging")\\
        .published_between(2022, 2024)\\
        .free_full_text_only()\\
        .limit(20)

    articles = client.pubmed.search_and_fetch(query, 0)
    print(f"Step 1: Found {len(articles)} articles")

    # Step 2: Check PMC availability
    pmids = [int(a.pmid) for a in articles if a.pmid.isdigit()]
    pmc_links = client.get_pmc_links(pmids)
    print(f"Step 2: {len(pmc_links)} articles have PMC full-text")

    # Step 3: Download full-text and extract figures
    output_dir = "./deep_learning_medical"
    os.makedirs(output_dir, exist_ok=True)

    for pmc_id in pmc_links.pmc_ids[:5]:  # First 5
        try:
            # Fetch full-text
            full_text = client.pmc.fetch_full_text(pmc_id)

            # Save markdown
            md_file = f"{output_dir}/{pmc_id}.md"
            with open(md_file, "w", encoding="utf-8") as f:
                f.write(full_text.to_markdown())

            # Extract figures
            fig_dir = f"{output_dir}/{pmc_id}_figures"
            figures = client.pmc.extract_figures_with_captions(pmc_id, fig_dir)

            print(f"Step 3: Processed {pmc_id}")
            print(f"  - Saved markdown to {md_file}")
            print(f"  - Extracted {len(figures)} figures to {fig_dir}")

        except Exception as e:
            print(f"Failed to process {pmc_id}: {e}")

    print("\\nWorkflow complete!")
