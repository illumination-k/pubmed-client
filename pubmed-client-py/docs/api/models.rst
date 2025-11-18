Data Models
===========

This page documents all data models used in the pubmed-client-py library.

PubMed Models
-------------

See :doc:`pubmed` for detailed documentation of PubMed-specific models:

- :class:`PubMedArticle` - Article metadata
- :class:`Author` - Author information
- :class:`Affiliation` - Author affiliation
- :class:`RelatedArticles` - Related articles from ELink
- :class:`PmcLinks` - PMC availability links
- :class:`Citations` - Citation information
- :class:`DatabaseInfo` - NCBI database information

PMC Models
----------

See :doc:`pmc` for detailed documentation of PMC-specific models:

- :class:`PmcFullText` - Full-text article
- :class:`PmcAuthor` - PMC author information
- :class:`PmcAffiliation` - PMC affiliation information
- :class:`ArticleSection` - Article section
- :class:`Figure` - Figure metadata
- :class:`ExtractedFigure` - Extracted figure with file information
- :class:`Table` - Table metadata
- :class:`Reference` - Bibliographic reference

Configuration Models
--------------------

See :doc:`client` for detailed documentation of configuration models:

- :class:`ClientConfig` - Client configuration with builder pattern

Query Models
------------

See :doc:`query` for detailed documentation of query building:

- :class:`SearchQuery` - Query builder for PubMed searches

Model Hierarchies
-----------------

Author Models
~~~~~~~~~~~~~

**PubMed Author:**

.. code-block:: python

   Author
   ├── last_name: Optional[str]
   ├── fore_name: Optional[str]
   ├── first_name: Optional[str]
   ├── middle_name: Optional[str]
   ├── initials: Optional[str]
   ├── suffix: Optional[str]
   ├── full_name: str
   ├── orcid: Optional[str]
   ├── is_corresponding: bool
   └── affiliations() -> list[Affiliation]

**PMC Author:**

.. code-block:: python

   PmcAuthor
   ├── given_names: Optional[str]
   ├── surname: Optional[str]
   ├── full_name: str
   ├── orcid: Optional[str]
   ├── email: Optional[str]
   ├── is_corresponding: bool
   └── affiliations() -> list[PmcAffiliation]

Article Models
~~~~~~~~~~~~~~

**PubMed Article (Metadata):**

.. code-block:: python

   PubMedArticle
   ├── pmid: str
   ├── title: str
   ├── journal: str
   ├── pub_date: str
   ├── doi: Optional[str]
   ├── pmc_id: Optional[str]
   ├── abstract_text: Optional[str]
   ├── author_count: int
   ├── authors() -> list[Author]
   ├── article_types() -> list[str]
   └── keywords() -> list[str]

**PMC Article (Full-Text):**

.. code-block:: python

   PmcFullText
   ├── pmcid: str
   ├── pmid: Optional[str]
   ├── title: str
   ├── doi: Optional[str]
   ├── authors() -> list[PmcAuthor]
   ├── sections() -> list[ArticleSection]
   ├── figures() -> list[Figure]
   ├── tables() -> list[Table]
   ├── references() -> list[Reference]
   └── to_markdown() -> str

Link Models
~~~~~~~~~~~

.. code-block:: python

   RelatedArticles
   ├── source_pmids: list[int]
   ├── related_pmids: list[int]
   ├── link_type: str
   └── __len__() -> int

   PmcLinks
   ├── source_pmids: list[int]
   ├── pmc_ids: list[str]
   └── __len__() -> int

   Citations
   ├── source_pmids: list[int]
   ├── citing_pmids: list[int]
   └── __len__() -> int

Type Annotations
----------------

All models include complete type annotations for use with mypy and type checkers:

.. code-block:: python

   from pubmed_client import Client, PubMedArticle, PmcFullText
   from typing import Optional

   def process_article(article: PubMedArticle) -> None:
       title: str = article.title
       pmid: str = article.pmid
       doi: Optional[str] = article.doi

   def process_full_text(full_text: PmcFullText) -> str:
       markdown: str = full_text.to_markdown()
       return markdown

String Representations
----------------------

All models implement ``__repr__()`` for debugging:

.. code-block:: python

   article = client.pubmed.fetch_article("31978945")
   print(repr(article))
   # PubMedArticle(pmid='31978945', title='Clinical features of patients...')

   author = article.authors()[0]
   print(repr(author))
   # Author(full_name='Huang C', orcid=None, is_corresponding=False)

Common Patterns
---------------

Checking for Optional Fields
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: python

   article = client.pubmed.fetch_article("31978945")

   # Check for DOI
   if article.doi:
       print(f"DOI: {article.doi}")

   # Check for PMC ID
   if article.pmc_id:
       full_text = client.pmc.fetch_full_text(article.pmc_id)

   # Check for abstract
   if article.abstract_text:
       print(f"Abstract: {article.abstract_text[:200]}...")

Iterating Over Collections
~~~~~~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: python

   full_text = client.pmc.fetch_full_text("PMC7906746")

   # Iterate authors
   for author in full_text.authors():
       print(f"{author.full_name} ({author.email})")

   # Iterate sections
   for section in full_text.sections():
       print(f"## {section.title}")
       print(section.content)

   # Iterate figures
   for figure in full_text.figures():
       print(f"{figure.label}: {figure.caption}")

   # Iterate references
   for ref in full_text.references():
       if ref.pmid:
           print(f"PMID: {ref.pmid} - {ref.title}")

Working with Extracted Figures
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: python

   figures = client.pmc.extract_figures_with_captions("PMC7906746", "./output")

   for fig in figures:
       # Access metadata
       metadata: Figure = fig.figure
       print(f"ID: {metadata.id}")
       print(f"Caption: {metadata.caption}")

       # Access file information
       path: str = fig.extracted_file_path
       size: Optional[int] = fig.file_size
       dims: Optional[tuple[int, int]] = fig.dimensions

       if dims:
           width, height = dims
           print(f"Image: {width}x{height} pixels, {size} bytes")
