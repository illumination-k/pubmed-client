"""Tests for data model classes and their properties."""

import pytest

import pubmed_client


@pytest.mark.integration
class TestPubMedArticleModel:
    """Tests for PubMedArticle data model."""

    def test_article_properties(self, pubmed_client: pubmed_client.PubMedClient) -> None:
        """Test article properties and methods."""
        article = pubmed_client.fetch_article("31978945")

        # Test basic properties
        assert hasattr(article, "pmid")
        assert hasattr(article, "title")
        assert hasattr(article, "journal")
        assert hasattr(article, "pub_date")
        assert hasattr(article, "doi")
        assert hasattr(article, "abstract_text")
        assert hasattr(article, "author_count")

        # Test methods
        authors = article.authors()
        assert isinstance(authors, list)

        article_types = article.article_types()
        assert isinstance(article_types, list)

        keywords = article.keywords()
        # keywords can be None or a list
        assert keywords is None or isinstance(keywords, list)

        # Test __repr__
        repr_str = repr(article)
        assert "PubMedArticle" in repr_str
        assert article.pmid in repr_str


@pytest.mark.integration
class TestAuthorModel:
    """Tests for Author data model."""

    def test_author_properties(self, pubmed_client: pubmed_client.PubMedClient) -> None:
        """Test author properties."""
        article = pubmed_client.fetch_article("31978945")
        authors = article.authors()

        if len(authors) > 0:
            author = authors[0]

            # Test basic properties
            assert hasattr(author, "full_name")
            assert hasattr(author, "surname")
            assert hasattr(author, "given_names")
            assert hasattr(author, "initials")
            assert hasattr(author, "suffix")
            assert hasattr(author, "orcid")
            assert hasattr(author, "email")
            assert hasattr(author, "is_corresponding")

            # Test affiliations method
            affiliations = author.affiliations()
            assert isinstance(affiliations, list)

            # Test roles method
            roles = author.roles()
            assert isinstance(roles, list)

            # Test __repr__
            repr_str = repr(author)
            assert "Author" in repr_str
            assert author.full_name in repr_str


@pytest.mark.integration
class TestPmcFullTextModel:
    """Tests for PmcFullText data model."""

    def test_full_text_properties(self, pmc_client: pubmed_client.PmcClient) -> None:
        """Test full text properties and methods."""
        full_text = pmc_client.fetch_full_text("PMC7906746")

        # Test basic properties
        assert hasattr(full_text, "pmcid")
        assert hasattr(full_text, "pmid")
        assert hasattr(full_text, "title")
        assert hasattr(full_text, "doi")

        assert full_text.pmcid == "PMC7906746"

        # Test methods
        authors = full_text.authors()
        assert isinstance(authors, list)

        sections = full_text.sections()
        assert isinstance(sections, list)

        figures = full_text.figures()
        assert isinstance(figures, list)

        tables = full_text.tables()
        assert isinstance(tables, list)

        references = full_text.references()
        assert isinstance(references, list)

        # Test __repr__
        repr_str = repr(full_text)
        assert "PmcFullText" in repr_str
        assert full_text.pmcid in repr_str


@pytest.mark.integration
class TestPmcAuthorModel:
    """Tests for PMC Author data model."""

    def test_pmc_author_properties(self, pmc_client: pubmed_client.PmcClient) -> None:
        """Test PMC author properties."""
        full_text = pmc_client.fetch_full_text("PMC7906746")
        authors = full_text.authors()

        if len(authors) > 0:
            author = authors[0]

            # Test basic properties
            assert hasattr(author, "full_name")
            assert hasattr(author, "given_names")
            assert hasattr(author, "surname")
            assert hasattr(author, "orcid")
            assert hasattr(author, "email")
            assert hasattr(author, "is_corresponding")

            # Test affiliations method
            affiliations = author.affiliations()
            assert isinstance(affiliations, list)

            # Test roles method
            roles = author.roles()
            assert isinstance(roles, list)

            # Test __repr__
            repr_str = repr(author)
            assert "PmcAuthor" in repr_str
            assert author.full_name in repr_str


@pytest.mark.integration
class TestFigureModel:
    """Tests for Figure data model."""

    def test_figure_properties(self, pmc_client: pubmed_client.PmcClient) -> None:
        """Test figure properties."""
        full_text = pmc_client.fetch_full_text("PMC7906746")
        figures = full_text.figures()

        if len(figures) > 0:
            figure = figures[0]

            # Test basic properties
            assert hasattr(figure, "id")
            assert hasattr(figure, "label")
            assert hasattr(figure, "caption")
            assert hasattr(figure, "alt_text")
            assert hasattr(figure, "fig_type")
            assert hasattr(figure, "file_path")
            assert hasattr(figure, "file_name")

            # Test __repr__
            repr_str = repr(figure)
            assert "Figure" in repr_str
            assert figure.id in repr_str


@pytest.mark.integration
class TestReferenceModel:
    """Tests for Reference data model."""

    def test_reference_properties(self, pmc_client: pubmed_client.PmcClient) -> None:
        """Test reference properties."""
        full_text = pmc_client.fetch_full_text("PMC7906746")
        references = full_text.references()

        if len(references) > 0:
            reference = references[0]

            # Test basic properties
            assert hasattr(reference, "id")
            assert hasattr(reference, "title")
            assert hasattr(reference, "journal")
            assert hasattr(reference, "year")
            assert hasattr(reference, "pmid")
            assert hasattr(reference, "doi")

            # Test __repr__
            repr_str = repr(reference)
            assert "Reference" in repr_str
            assert reference.id in repr_str


@pytest.mark.integration
class TestRelatedArticlesModel:
    """Tests for RelatedArticles data model."""

    def test_related_articles_properties(self, pubmed_client: pubmed_client.PubMedClient) -> None:
        """Test related articles properties."""
        related = pubmed_client.get_related_articles([31978945])

        # Test basic properties
        assert hasattr(related, "source_pmids")
        assert hasattr(related, "related_pmids")
        assert hasattr(related, "link_type")

        assert isinstance(related.source_pmids, list)
        assert isinstance(related.related_pmids, list)
        assert isinstance(related.link_type, str)

        # Test __len__
        assert len(related) == len(related.related_pmids)

        # Test __repr__
        repr_str = repr(related)
        assert "RelatedArticles" in repr_str


@pytest.mark.integration
class TestPmcLinksModel:
    """Tests for PmcLinks data model."""

    def test_pmc_links_properties(self, pubmed_client: pubmed_client.PubMedClient) -> None:
        """Test PMC links properties."""
        links = pubmed_client.get_pmc_links([31978945])

        # Test basic properties
        assert hasattr(links, "source_pmids")
        assert hasattr(links, "pmc_ids")

        assert isinstance(links.source_pmids, list)
        assert isinstance(links.pmc_ids, list)

        # Test __len__
        assert len(links) == len(links.pmc_ids)

        # Test __repr__
        repr_str = repr(links)
        assert "PmcLinks" in repr_str


@pytest.mark.integration
class TestCitationsModel:
    """Tests for Citations data model."""

    def test_citations_properties(self, pubmed_client: pubmed_client.PubMedClient) -> None:
        """Test citations properties."""
        citations = pubmed_client.get_citations([31978945])

        # Test basic properties
        assert hasattr(citations, "source_pmids")
        assert hasattr(citations, "citing_pmids")

        assert isinstance(citations.source_pmids, list)
        assert isinstance(citations.citing_pmids, list)

        # Test __len__
        assert len(citations) == len(citations.citing_pmids)

        # Test __repr__
        repr_str = repr(citations)
        assert "Citations" in repr_str
