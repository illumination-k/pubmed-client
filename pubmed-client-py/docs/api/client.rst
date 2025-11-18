Client API
==========

The Client class provides the main interface for interacting with PubMed and PMC APIs.

Client
------

.. class:: pubmed_client.Client

   Combined client with both PubMed and PMC functionality.

   This is the main client you'll typically use. It provides access to both
   PubMed metadata searches and PMC full-text retrieval.

   **Attributes:**

   .. attribute:: pubmed
      :type: PubMedClient

      Get PubMed client for metadata operations

   .. attribute:: pmc
      :type: PmcClient

      Get PMC client for full-text operations

   **Methods:**

   .. method:: __init__() -> Client

      Create a new combined client with default configuration

      :return: New Client instance
      :rtype: Client

      **Example:**

      .. code-block:: python

         client = pubmed_client.Client()

   .. staticmethod:: with_config(config: ClientConfig) -> Client

      Create a new combined client with custom configuration

      :param config: Client configuration
      :type config: ClientConfig
      :return: New Client instance
      :rtype: Client

      **Example:**

      .. code-block:: python

         config = pubmed_client.ClientConfig()\\
             .with_api_key("your_key")\\
             .with_email("you@example.com")
         client = pubmed_client.Client.with_config(config)

   .. method:: search_with_full_text(query: str, limit: int) -> list[tuple[PubMedArticle, Optional[PmcFullText]]]

      Search for articles and attempt to fetch full text for each

      This is a convenience method that searches PubMed and attempts to fetch
      PMC full text for each result when available.

      :param query: Search query string
      :type query: str
      :param limit: Maximum number of articles to process
      :type limit: int
      :return: List of tuples (PubMedArticle, Optional[PmcFullText])
      :rtype: list[tuple[PubMedArticle, Optional[PmcFullText]]]

      **Example:**

      .. code-block:: python

         results = client.search_with_full_text("covid-19", 5)
         for article, full_text in results:
             print(article.title)
             if full_text:
                 print(f"  PMC: {full_text.pmcid}")

   .. method:: get_database_list() -> list[str]

      Get list of all available NCBI databases

      :return: List of database names
      :rtype: list[str]

   .. method:: get_database_info(database: str) -> DatabaseInfo

      Get detailed information about a specific database

      :param database: Database name (e.g., "pubmed", "pmc")
      :type database: str
      :return: Database information
      :rtype: DatabaseInfo

   .. method:: get_related_articles(pmids: Sequence[int]) -> RelatedArticles

      Get related articles for given PMIDs

      :param pmids: List of PubMed IDs
      :type pmids: Sequence[int]
      :return: Related articles information
      :rtype: RelatedArticles

   .. method:: get_pmc_links(pmids: Sequence[int]) -> PmcLinks

      Get PMC links for given PMIDs

      :param pmids: List of PubMed IDs
      :type pmids: Sequence[int]
      :return: PMC links information
      :rtype: PmcLinks

   .. method:: get_citations(pmids: Sequence[int]) -> Citations

      Get citing articles for given PMIDs

      :param pmids: List of PubMed IDs
      :type pmids: Sequence[int]
      :return: Citations information
      :rtype: Citations

ClientConfig
------------

.. class:: pubmed_client.ClientConfig

   Configuration for PubMed and PMC clients.

   Supports builder pattern for method chaining.

   **Methods:**

   .. method:: __init__() -> ClientConfig

      Create a new configuration with default settings

      :return: New ClientConfig instance
      :rtype: ClientConfig

   .. method:: with_api_key(api_key: str) -> ClientConfig

      Set the NCBI API key for increased rate limits (10 req/sec instead of 3)

      :param api_key: NCBI API key
      :type api_key: str
      :return: Self for method chaining
      :rtype: ClientConfig

   .. method:: with_email(email: str) -> ClientConfig

      Set the email address for identification (recommended by NCBI)

      :param email: Email address
      :type email: str
      :return: Self for method chaining
      :rtype: ClientConfig

   .. method:: with_tool(tool: str) -> ClientConfig

      Set the tool name for identification

      :param tool: Tool name (default: "pubmed-client-py")
      :type tool: str
      :return: Self for method chaining
      :rtype: ClientConfig

   .. method:: with_rate_limit(rate_limit: float) -> ClientConfig

      Set custom rate limit in requests per second

      :param rate_limit: Rate limit (requests per second)
      :type rate_limit: float
      :return: Self for method chaining
      :rtype: ClientConfig

   .. method:: with_timeout_seconds(timeout_seconds: int) -> ClientConfig

      Set HTTP request timeout in seconds

      :param timeout_seconds: Timeout in seconds
      :type timeout_seconds: int
      :return: Self for method chaining
      :rtype: ClientConfig

   .. method:: with_cache() -> ClientConfig

      Enable default response caching

      :return: Self for method chaining
      :rtype: ClientConfig

   **Example:**

   .. code-block:: python

      config = pubmed_client.ClientConfig()\\
          .with_api_key("your_api_key")\\
          .with_email("you@example.com")\\
          .with_tool("MyResearchTool")\\
          .with_rate_limit(10.0)\\
          .with_timeout_seconds(30)\\
          .with_cache()

      client = pubmed_client.Client.with_config(config)
