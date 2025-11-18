PMC API
=======

The PmcClient provides access to PubMed Central full-text articles.

PmcClient
---------

.. class:: pubmed_client.PmcClient

   PMC client for fetching full-text articles.

   **Methods:**

   .. method:: __init__() -> PmcClient

      Create a new PMC client with default configuration

      :return: New PmcClient instance
      :rtype: PmcClient

   .. staticmethod:: with_config(config: ClientConfig) -> PmcClient

      Create a new PMC client with custom configuration

      :param config: Client configuration
      :type config: ClientConfig
      :return: New PmcClient instance
      :rtype: PmcClient

   .. method:: fetch_full_text(pmcid: str) -> PmcFullText

      Fetch full text article from PMC

      :param pmcid: PMC ID (e.g., "PMC7906746")
      :type pmcid: str
      :return: PmcFullText object containing structured article content
      :rtype: PmcFullText

      **Example:**

      .. code-block:: python

         client = pubmed_client.PmcClient()
         full_text = client.fetch_full_text("PMC7906746")
         print(full_text.title)

   .. method:: check_pmc_availability(pmid: str) -> Optional[str]

      Check if PMC full text is available for a PMID

      :param pmid: PubMed ID as a string
      :type pmid: str
      :return: PMC ID if available, None otherwise
      :rtype: Optional[str]

      **Example:**

      .. code-block:: python

         pmcid = client.check_pmc_availability("31978945")
         if pmcid:
             print(f"Available: {pmcid}")

   .. method:: download_and_extract_tar(pmcid: str, output_dir: str) -> list[str]

      Download and extract PMC tar.gz archive

      Downloads the tar.gz file for the specified PMC ID and extracts all files
      to the output directory.

      :param pmcid: PMC ID (e.g., "PMC7906746" or "7906746")
      :type pmcid: str
      :param output_dir: Directory path where files should be extracted
      :type output_dir: str
      :return: List of extracted file paths
      :rtype: list[str]

      **Example:**

      .. code-block:: python

         files = client.download_and_extract_tar("PMC7906746", "./output")
         for file in files:
             print(file)

   .. method:: extract_figures_with_captions(pmcid: str, output_dir: str) -> list[ExtractedFigure]

      Extract figures with captions from PMC article

      Downloads the tar.gz file for the specified PMC ID, extracts all files,
      and matches figures with their captions from the XML metadata.

      :param pmcid: PMC ID (e.g., "PMC7906746" or "7906746")
      :type pmcid: str
      :param output_dir: Directory path where files should be extracted
      :type output_dir: str
      :return: List of ExtractedFigure objects containing metadata and file information
      :rtype: list[ExtractedFigure]

      **Example:**

      .. code-block:: python

         figures = client.extract_figures_with_captions("PMC7906746", "./output")
         for fig in figures:
             print(f"{fig.figure.id}: {fig.extracted_file_path}")
             print(f"  Caption: {fig.figure.caption}")
             print(f"  Size: {fig.file_size} bytes")
             print(f"  Dimensions: {fig.dimensions}")

PmcFullText
-----------

.. class:: pubmed_client.PmcFullText

   Represents a full-text article from PMC.

   **Attributes:**

   .. attribute:: pmcid
      :type: str

      PMC ID

   .. attribute:: pmid
      :type: Optional[str]

      PubMed ID if available

   .. attribute:: title
      :type: str

      Article title

   .. attribute:: doi
      :type: Optional[str]

      DOI (Digital Object Identifier)

   **Methods:**

   .. method:: authors() -> list[PmcAuthor]

      Get list of authors

      :return: List of PmcAuthor objects
      :rtype: list[PmcAuthor]

   .. method:: sections() -> list[ArticleSection]

      Get list of sections

      :return: List of ArticleSection objects
      :rtype: list[ArticleSection]

   .. method:: figures() -> list[Figure]

      Get list of all figures from all sections

      :return: List of Figure objects
      :rtype: list[Figure]

   .. method:: tables() -> list[Table]

      Get list of all tables from all sections

      :return: List of Table objects
      :rtype: list[Table]

   .. method:: references() -> list[Reference]

      Get list of references

      :return: List of Reference objects
      :rtype: list[Reference]

   .. method:: to_markdown() -> str

      Convert the article to Markdown format

      :return: A Markdown-formatted string representation of the article
      :rtype: str

      **Example:**

      .. code-block:: python

         full_text = client.pmc.fetch_full_text("PMC7906746")
         markdown = full_text.to_markdown()
         with open("article.md", "w") as f:
             f.write(markdown)

PmcAuthor
---------

.. class:: pubmed_client.PmcAuthor

   Represents a PMC article author.

   **Attributes:**

   .. attribute:: given_names
      :type: Optional[str]

      Given names

   .. attribute:: surname
      :type: Optional[str]

      Surname

   .. attribute:: full_name
      :type: str

      Full name

   .. attribute:: orcid
      :type: Optional[str]

      ORCID identifier

   .. attribute:: email
      :type: Optional[str]

      Email address

   .. attribute:: is_corresponding
      :type: bool

      Whether this is a corresponding author

   **Methods:**

   .. method:: affiliations() -> list[PmcAffiliation]

      Get list of affiliations

      :return: List of PmcAffiliation objects
      :rtype: list[PmcAffiliation]

PmcAffiliation
--------------

.. class:: pubmed_client.PmcAffiliation

   Represents a PMC author's affiliation.

   **Attributes:**

   .. attribute:: id
      :type: Optional[str]

      Affiliation ID

   .. attribute:: institution
      :type: str

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

ArticleSection
--------------

.. class:: pubmed_client.ArticleSection

   Represents a section of an article.

   **Attributes:**

   .. attribute:: title
      :type: Optional[str]

      Section title

   .. attribute:: content
      :type: str

      Section content

   .. attribute:: section_type
      :type: Optional[str]

      Type of section

Figure
------

.. class:: pubmed_client.Figure

   Represents a figure in an article.

   **Attributes:**

   .. attribute:: id
      :type: str

      Figure ID

   .. attribute:: label
      :type: Optional[str]

      Figure label (e.g., "Figure 1")

   .. attribute:: caption
      :type: str

      Figure caption

   .. attribute:: alt_text
      :type: Optional[str]

      Alternative text

   .. attribute:: fig_type
      :type: Optional[str]

      Figure type

   .. attribute:: file_path
      :type: Optional[str]

      File path from XML

   .. attribute:: file_name
      :type: Optional[str]

      File name

ExtractedFigure
---------------

.. class:: pubmed_client.ExtractedFigure

   Represents a figure that has been extracted from a PMC tar.gz archive,
   combining XML metadata with actual file information.

   **Attributes:**

   .. attribute:: figure
      :type: Figure

      Figure metadata from XML (caption, label, etc.)

   .. attribute:: extracted_file_path
      :type: str

      Actual file path where the figure was extracted

   .. attribute:: file_size
      :type: Optional[int]

      File size in bytes

   .. attribute:: dimensions
      :type: Optional[tuple[int, int]]

      Image dimensions as (width, height) tuple if available

Table
-----

.. class:: pubmed_client.Table

   Represents a table in an article.

   **Attributes:**

   .. attribute:: id
      :type: str

      Table ID

   .. attribute:: label
      :type: Optional[str]

      Table label (e.g., "Table 1")

   .. attribute:: caption
      :type: str

      Table caption

Reference
---------

.. class:: pubmed_client.Reference

   Represents a bibliographic reference.

   **Attributes:**

   .. attribute:: id
      :type: str

      Reference ID

   .. attribute:: title
      :type: Optional[str]

      Article title

   .. attribute:: journal
      :type: Optional[str]

      Journal name

   .. attribute:: year
      :type: Optional[str]

      Publication year

   .. attribute:: pmid
      :type: Optional[str]

      PubMed ID

   .. attribute:: doi
      :type: Optional[str]

      DOI
