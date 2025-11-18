PubMed API
==========

The PubMedClient provides access to PubMed article metadata and search functionality.

PubMedClient
------------

.. class:: pubmed_client.PubMedClient

   PubMed client for searching and fetching article metadata.

   **Methods:**

   .. method:: __init__() -> PubMedClient

      Create a new PubMed client with default configuration

      :return: New PubMedClient instance
      :rtype: PubMedClient

   .. staticmethod:: with_config(config: ClientConfig) -> PubMedClient

      Create a new PubMed client with custom configuration

      :param config: Client configuration
      :type config: ClientConfig
      :return: New PubMedClient instance
      :rtype: PubMedClient

   .. method:: search_articles(query: str | SearchQuery, limit: int) -> list[str]

      Search for articles and return PMIDs only

      This method returns only the list of PMIDs matching the query,
      which is faster than fetching full article metadata.

      :param query: Search query (either a string or SearchQuery object)
      :type query: str | SearchQuery
      :param limit: Maximum number of PMIDs to return (ignored if query is SearchQuery)
      :type limit: int
      :return: List of PMIDs as strings
      :rtype: list[str]

      **Example:**

      .. code-block:: python

         client = pubmed_client.PubMedClient()

         # Using string query
         pmids = client.search_articles("covid-19", 100)

         # Using SearchQuery object
         query = SearchQuery().query("covid-19").limit(100)
         pmids = client.search_articles(query, 0)

   .. method:: search_and_fetch(query: str | SearchQuery, limit: int) -> list[PubMedArticle]

      Search for articles and fetch their metadata

      :param query: Search query (either a string or SearchQuery object)
      :type query: str | SearchQuery
      :param limit: Maximum number of articles to return (ignored if query is SearchQuery)
      :type limit: int
      :return: List of PubMedArticle objects
      :rtype: list[PubMedArticle]

      **Example:**

      .. code-block:: python

         # Using string query
         articles = client.search_and_fetch("covid-19", 10)

         # Using SearchQuery object
         query = SearchQuery().query("cancer").published_after(2020).limit(50)
         articles = client.search_and_fetch(query, 0)

   .. method:: fetch_article(pmid: str) -> PubMedArticle

      Fetch a single article by PMID

      :param pmid: PubMed ID as a string
      :type pmid: str
      :return: PubMedArticle object
      :rtype: PubMedArticle

   .. method:: get_database_list() -> list[str]

      Get list of all available NCBI databases

      :return: List of database names
      :rtype: list[str]

   .. method:: get_database_info(database: str) -> DatabaseInfo

      Get detailed information about a specific database

      :param database: Database name (e.g., "pubmed", "pmc")
      :type database: str
      :return: DatabaseInfo object
      :rtype: DatabaseInfo

   .. method:: get_related_articles(pmids: Sequence[int]) -> RelatedArticles

      Get related articles for given PMIDs

      :param pmids: List of PubMed IDs
      :type pmids: Sequence[int]
      :return: RelatedArticles object
      :rtype: RelatedArticles

   .. method:: get_pmc_links(pmids: Sequence[int]) -> PmcLinks

      Get PMC links for given PMIDs (full-text availability)

      :param pmids: List of PubMed IDs
      :type pmids: Sequence[int]
      :return: PmcLinks object containing available PMC IDs
      :rtype: PmcLinks

   .. method:: get_citations(pmids: Sequence[int]) -> Citations

      Get citing articles for given PMIDs

      Returns articles that cite the specified PMIDs from the PubMed database only.

      .. note::
         Citation counts from this method may be LOWER than Google Scholar
         or scite.ai because this only includes peer-reviewed articles in PubMed.
         Other sources include preprints, books, and conference proceedings.

      :param pmids: List of PubMed IDs
      :type pmids: Sequence[int]
      :return: Citations object containing citing article PMIDs
      :rtype: Citations

PubMedArticle
-------------

.. class:: pubmed_client.PubMedArticle

   Represents a PubMed article with metadata.

   **Attributes:**

   .. attribute:: pmid
      :type: str

      PubMed ID

   .. attribute:: title
      :type: str

      Article title

   .. attribute:: journal
      :type: str

      Journal name

   .. attribute:: pub_date
      :type: str

      Publication date

   .. attribute:: doi
      :type: Optional[str]

      DOI (Digital Object Identifier)

   .. attribute:: pmc_id
      :type: Optional[str]

      PMC ID if available

   .. attribute:: abstract_text
      :type: Optional[str]

      Abstract text

   .. attribute:: author_count
      :type: int

      Number of authors

   **Methods:**

   .. method:: authors() -> list[Author]

      Get list of authors

      :return: List of Author objects
      :rtype: list[Author]

   .. method:: article_types() -> list[str]

      Get article types

      :return: List of article type strings
      :rtype: list[str]

   .. method:: keywords() -> list[str]

      Get keywords

      :return: List of keyword strings
      :rtype: list[str]

Author
------

.. class:: pubmed_client.Author

   Represents an article author.

   **Attributes:**

   .. attribute:: last_name
      :type: Optional[str]

      Author's last name

   .. attribute:: fore_name
      :type: Optional[str]

      Author's forename

   .. attribute:: first_name
      :type: Optional[str]

      Author's first name

   .. attribute:: middle_name
      :type: Optional[str]

      Author's middle name

   .. attribute:: initials
      :type: Optional[str]

      Author's initials

   .. attribute:: suffix
      :type: Optional[str]

      Author's suffix (e.g., Jr., Sr.)

   .. attribute:: full_name
      :type: str

      Author's full name

   .. attribute:: orcid
      :type: Optional[str]

      ORCID identifier

   .. attribute:: is_corresponding
      :type: bool

      Whether this is a corresponding author

   **Methods:**

   .. method:: affiliations() -> list[Affiliation]

      Get list of affiliations

      :return: List of Affiliation objects
      :rtype: list[Affiliation]

Affiliation
-----------

.. class:: pubmed_client.Affiliation

   Represents an author's affiliation.

   **Attributes:**

   .. attribute:: institution
      :type: Optional[str]

      Institution name

   .. attribute:: department
      :type: Optional[str]

      Department name

   .. attribute:: address
      :type: Optional[str]

      Address

   .. attribute:: country
      :type: Optional[str]

      Country

   .. attribute:: email
      :type: Optional[str]

      Email address

RelatedArticles
---------------

.. class:: pubmed_client.RelatedArticles

   Contains related article information from ELink API.

   **Attributes:**

   .. attribute:: source_pmids
      :type: list[int]

      Source PubMed IDs

   .. attribute:: related_pmids
      :type: list[int]

      Related PubMed IDs

   .. attribute:: link_type
      :type: str

      Type of relationship link

   **Methods:**

   .. method:: __len__() -> int

      Get number of related articles

PmcLinks
--------

.. class:: pubmed_client.PmcLinks

   Contains PMC availability information.

   **Attributes:**

   .. attribute:: source_pmids
      :type: list[int]

      Source PubMed IDs

   .. attribute:: pmc_ids
      :type: list[str]

      Available PMC IDs

   **Methods:**

   .. method:: __len__() -> int

      Get number of PMC links

Citations
---------

.. class:: pubmed_client.Citations

   Contains citation information from ELink API.

   **Attributes:**

   .. attribute:: source_pmids
      :type: list[int]

      Source PubMed IDs

   .. attribute:: citing_pmids
      :type: list[int]

      Citing PubMed IDs

   **Methods:**

   .. method:: __len__() -> int

      Get number of citations

DatabaseInfo
------------

.. class:: pubmed_client.DatabaseInfo

   Contains NCBI database information from EInfo API.

   **Attributes:**

   .. attribute:: name
      :type: str

      Database name

   .. attribute:: menu_name
      :type: str

      Menu display name

   .. attribute:: description
      :type: str

      Database description

   .. attribute:: build
      :type: Optional[str]

      Build version

   .. attribute:: count
      :type: Optional[int]

      Number of records

   .. attribute:: last_update
      :type: Optional[str]

      Last update date
