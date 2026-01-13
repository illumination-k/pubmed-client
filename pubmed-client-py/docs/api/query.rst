Query Builder API
=================

The SearchQuery class provides a builder pattern for constructing complex PubMed search queries.

SearchQuery
-----------

.. class:: pubmed_client.SearchQuery

   Builder for constructing PubMed search queries programmatically.

   **Constructor:**

   .. method:: __init__() -> SearchQuery

      Create a new empty search query builder

      :return: New query builder instance
      :rtype: SearchQuery

      **Example:**

      .. code-block:: python

         query = SearchQuery()

Basic Query Methods
-------------------

.. method:: SearchQuery.query(term: Optional[str] = None) -> SearchQuery

   Add a search term to the query

   Terms are accumulated (not replaced) and will be space-separated in the final query.
   None and empty strings (after trimming) are silently filtered out.

   :param term: Search term string (None or empty strings are filtered)
   :type term: Optional[str]
   :return: Self for method chaining
   :rtype: SearchQuery

   **Example:**

   .. code-block:: python

      query = SearchQuery().query("covid-19").query("treatment")
      query.build()  # Returns: 'covid-19 treatment'

.. method:: SearchQuery.terms(terms: Optional[Sequence[Optional[str]]] = None) -> SearchQuery

   Add multiple search terms at once

   Each term is processed like query(). None items and empty strings are filtered out.

   :param terms: List of search term strings
   :type terms: Optional[Sequence[Optional[str]]]
   :return: Self for method chaining
   :rtype: SearchQuery

   **Example:**

   .. code-block:: python

      query = SearchQuery().terms(["covid-19", "vaccine", "efficacy"])
      query.build()  # Returns: 'covid-19 vaccine efficacy'

.. method:: SearchQuery.limit(limit: Optional[int] = None) -> SearchQuery

   Set the maximum number of results to return

   Validates that limit is >0 and ≤10,000. None is treated as "use default" (20).

   :param limit: Maximum number of results (None = use default of 20)
   :type limit: Optional[int]
   :return: Self for method chaining
   :rtype: SearchQuery
   :raises ValueError: If limit ≤ 0 or limit > 10,000

   **Example:**

   .. code-block:: python

      query = SearchQuery().query("cancer").limit(50)

.. method:: SearchQuery.build() -> str

   Build the final PubMed query string

   Terms are joined with space separators (PubMed's default OR logic).

   :return: Query string for PubMed E-utilities API
   :rtype: str
   :raises ValueError: If no search terms have been added

   **Example:**

   .. code-block:: python

      query = SearchQuery().query("covid-19").query("treatment")
      query.build()  # Returns: 'covid-19 treatment'

.. method:: SearchQuery.get_limit() -> int

   Get the limit for this query

   Returns the configured limit or the default of 20 if not set.

   :return: Maximum number of results (default: 20)
   :rtype: int

   **Example:**

   .. code-block:: python

      query = SearchQuery().query("cancer").limit(100)
      query.get_limit()  # Returns: 100

Date Filter Methods
-------------------

.. method:: SearchQuery.published_in_year(year: int) -> SearchQuery

   Filter to articles published in a specific year

   :param year: Year to filter by (must be between 1800 and 3000)
   :type year: int
   :return: Self for method chaining
   :rtype: SearchQuery
   :raises ValueError: If year is outside the valid range (1800-3000)

   **Example:**

   .. code-block:: python

      query = SearchQuery().query("covid-19").published_in_year(2024)
      query.build()  # Returns: 'covid-19 AND 2024[pdat]'

.. method:: SearchQuery.published_between(start_year: int, end_year: Optional[int] = None) -> SearchQuery

   Filter by publication date range

   Filters articles published between start_year and end_year (inclusive).
   If end_year is None, filters from start_year onwards (up to year 3000).

   :param start_year: Start year (inclusive, must be 1800-3000)
   :type start_year: int
   :param end_year: End year (inclusive, optional, must be 1800-3000 if provided)
   :type end_year: Optional[int]
   :return: Self for method chaining
   :rtype: SearchQuery
   :raises ValueError: If years are outside valid range or start_year > end_year

   **Example:**

   .. code-block:: python

      # Filter to 2020-2024
      query = SearchQuery().query("cancer").published_between(2020, 2024)
      query.build()  # Returns: 'cancer AND 2020:2024[pdat]'

      # Filter from 2020 onwards
      query = SearchQuery().query("treatment").published_between(2020, None)
      query.build()  # Returns: 'treatment AND 2020:3000[pdat]'

.. method:: SearchQuery.published_after(year: int) -> SearchQuery

   Filter to articles published after a specific year

   Equivalent to published_between(year, None).

   :param year: Year after which articles were published (must be 1800-3000)
   :type year: int
   :return: Self for method chaining
   :rtype: SearchQuery
   :raises ValueError: If year is outside the valid range (1800-3000)

   **Example:**

   .. code-block:: python

      query = SearchQuery().query("crispr").published_after(2020)
      query.build()  # Returns: 'crispr AND 2020:3000[pdat]'

.. method:: SearchQuery.published_before(year: int) -> SearchQuery

   Filter to articles published before a specific year

   Filters articles from 1900 up to and including the specified year.

   :param year: Year before which articles were published (must be 1800-3000)
   :type year: int
   :return: Self for method chaining
   :rtype: SearchQuery
   :raises ValueError: If year is outside the valid range (1800-3000)

   **Example:**

   .. code-block:: python

      query = SearchQuery().query("genome").published_before(2020)
      query.build()  # Returns: 'genome AND 1900:2020[pdat]'

Article Type Filter Methods
----------------------------

.. method:: SearchQuery.article_type(type_name: str) -> SearchQuery

   Filter by a single article type

   :param type_name: Article type name (case-insensitive)
                     Supported types: "Clinical Trial", "Review", "Systematic Review",
                     "Meta-Analysis", "Case Reports", "Randomized Controlled Trial" (or "RCT"),
                     "Observational Study"
   :type type_name: str
   :return: Self for method chaining
   :rtype: SearchQuery
   :raises ValueError: If article type is not recognized

   **Example:**

   .. code-block:: python

      query = SearchQuery().query("cancer").article_type("Clinical Trial")
      query.build()  # Returns: 'cancer AND Clinical Trial[pt]'

.. method:: SearchQuery.article_types(types: Sequence[str]) -> SearchQuery

   Filter by multiple article types (OR logic)

   When multiple types are provided, they are combined with OR logic.
   Empty list is silently ignored (no filter added).

   :param types: List of article type names (case-insensitive)
   :type types: Sequence[str]
   :return: Self for method chaining
   :rtype: SearchQuery
   :raises ValueError: If any article type is not recognized

   **Example:**

   .. code-block:: python

      query = SearchQuery().query("treatment").article_types(["RCT", "Meta-Analysis"])
      query.build()  # Returns: 'treatment AND (Randomized Controlled Trial[pt] OR Meta-Analysis[pt])'

Full-Text Filter Methods
-------------------------

.. method:: SearchQuery.free_full_text_only() -> SearchQuery

   Filter to articles with free full text (open access)

   This includes articles that are freely available from PubMed Central
   and other open access sources.

   :return: Self for method chaining
   :rtype: SearchQuery

   **Example:**

   .. code-block:: python

      query = SearchQuery().query("cancer").free_full_text_only()
      query.build()  # Returns: 'cancer AND free full text[sb]'

.. method:: SearchQuery.full_text_only() -> SearchQuery

   Filter to articles with full text links

   This includes both free full text and subscription-based full text articles.
   Use free_full_text_only() if you only want open access articles.

   :return: Self for method chaining
   :rtype: SearchQuery

   **Example:**

   .. code-block:: python

      query = SearchQuery().query("diabetes").full_text_only()
      query.build()  # Returns: 'diabetes AND full text[sb]'

.. method:: SearchQuery.pmc_only() -> SearchQuery

   Filter to articles with PMC full text

   This filters to articles that have full text available in PubMed Central (PMC).

   :return: Self for method chaining
   :rtype: SearchQuery

   **Example:**

   .. code-block:: python

      query = SearchQuery().query("genomics").pmc_only()
      query.build()  # Returns: 'genomics AND pmc[sb]'

Boolean Logic Methods
---------------------

.. method:: SearchQuery.and_(other: SearchQuery) -> SearchQuery

   Combine this query with another using AND logic

   Combines two queries by wrapping each in parentheses and joining with AND.
   If either query is empty, returns the non-empty query.
   The result uses the higher limit of the two queries.

   :param other: Another SearchQuery to combine with
   :type other: SearchQuery
   :return: New query with combined logic
   :rtype: SearchQuery

   **Example:**

   .. code-block:: python

      q1 = SearchQuery().query("covid-19")
      q2 = SearchQuery().query("vaccine")
      combined = q1.and_(q2)
      combined.build()  # Returns: '(covid-19) AND (vaccine)'

.. method:: SearchQuery.or_(other: SearchQuery) -> SearchQuery

   Combine this query with another using OR logic

   Combines two queries by wrapping each in parentheses and joining with OR.
   If either query is empty, returns the non-empty query.
   The result uses the higher limit of the two queries.

   :param other: Another SearchQuery to combine with
   :type other: SearchQuery
   :return: New query with combined logic
   :rtype: SearchQuery

   **Example:**

   .. code-block:: python

      q1 = SearchQuery().query("diabetes")
      q2 = SearchQuery().query("hypertension")
      combined = q1.or_(q2)
      combined.build()  # Returns: '(diabetes) OR (hypertension)'

.. method:: SearchQuery.negate() -> SearchQuery

   Negate this query using NOT logic

   Wraps the current query with NOT operator.
   This is typically used in combination with other queries to exclude results.
   Returns an empty query if the current query is empty.

   :return: New query with NOT logic
   :rtype: SearchQuery

   **Example:**

   .. code-block:: python

      query = SearchQuery().query("cancer").negate()
      query.build()  # Returns: 'NOT (cancer)'

.. method:: SearchQuery.exclude(excluded: SearchQuery) -> SearchQuery

   Exclude articles matching the given query

   Excludes results from this query that match the excluded query.
   This is the recommended way to filter out unwanted results.
   If either query is empty, returns the base query unchanged.

   :param excluded: SearchQuery representing articles to exclude
   :type excluded: SearchQuery
   :return: New query with exclusion logic
   :rtype: SearchQuery

   **Example:**

   .. code-block:: python

      base = SearchQuery().query("cancer treatment")
      exclude = SearchQuery().query("animal studies")
      filtered = base.exclude(exclude)
      filtered.build()  # Returns: '(cancer treatment) NOT (animal studies)'

.. method:: SearchQuery.group() -> SearchQuery

   Add parentheses around the current query for grouping

   Wraps the query in parentheses to control operator precedence in complex queries.
   Returns an empty query if the current query is empty.

   :return: New query wrapped in parentheses
   :rtype: SearchQuery

   **Example:**

   .. code-block:: python

      query = SearchQuery().query("cancer").or_(SearchQuery().query("tumor")).group()
      query.build()  # Returns: '((cancer) OR (tumor))'

Complete Example
----------------

.. code-block:: python

   from pubmed_client import SearchQuery

   # Build a complex query
   cancer_q = SearchQuery().query("cancer").or_(SearchQuery().query("tumor"))
   treatment_q = SearchQuery().query("treatment").or_(SearchQuery().query("therapy"))
   review_q = SearchQuery().query("review[pt]")

   final_query = cancer_q\\
       .and_(treatment_q)\\
       .exclude(review_q)\\
       .published_between(2022, 2024)\\
       .free_full_text_only()\\
       .limit(100)

   # Use with client
   client = pubmed_client.Client()
   articles = client.pubmed.search_and_fetch(final_query, 0)
