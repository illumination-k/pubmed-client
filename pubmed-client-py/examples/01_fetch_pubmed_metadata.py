"""
Example: Retrieving PubMed Article Metadata from PMID

This example demonstrates how to fetch article metadata from PubMed using a PMID.
It shows how to access various metadata fields including title, authors, journal,
publication date, DOI, abstract, and keywords.

Usage:
    python 01_fetch_pubmed_metadata.py 31978945
    python 01_fetch_pubmed_metadata.py 31978945 33515491 25760099
    python 01_fetch_pubmed_metadata.py 31978945 --api-key YOUR_KEY --email you@example.com
"""

import argparse

import pubmed_client


def display_article(article: pubmed_client.PubMedArticle) -> None:
    """Display article metadata in a formatted way"""
    print("=" * 80)
    print(f"Title: {article.title}")
    print(f"Journal: {article.journal}")
    print(f"Publication Date: {article.pub_date}")
    print(f"PMID: {article.pmid}")

    if article.doi:
        print(f"DOI: {article.doi}")

    print(f"\nNumber of Authors: {article.author_count}")

    # Display authors
    print("\nAuthors:")
    for i, author in enumerate(article.authors(), start=1):
        author_info = f"{i}. {author.full_name}"

        # Add ORCID if available
        if author.orcid:
            author_info += f" (ORCID: {author.orcid})"

        # Mark corresponding author
        if author.is_corresponding:
            author_info += " [Corresponding Author]"

        print(f"  {author_info}")

        # Display author's affiliations
        affiliations = author.affiliations()
        if affiliations:
            for j, aff in enumerate(affiliations, start=1):
                aff_parts = []
                if aff.department:
                    aff_parts.append(aff.department)
                if aff.institution:
                    aff_parts.append(aff.institution)
                if aff.country:
                    aff_parts.append(aff.country)

                if aff_parts:
                    print(f"     Affiliation {j}: {', '.join(aff_parts)}")

    # Display article types
    article_types = article.article_types()
    if article_types:
        print(f"\nArticle Types: {', '.join(article_types)}")

    # Display keywords if available
    keywords = article.keywords()
    if keywords:
        print(f"\nKeywords ({len(keywords)}): {', '.join(keywords)}")

    # Display abstract
    if article.abstract_text:
        print(f"\nAbstract ({len(article.abstract_text)} characters):")
        print("-" * 80)
        # Display first 500 characters of abstract
        abstract_preview = (
            article.abstract_text[:500] + "..."
            if len(article.abstract_text) > 500
            else article.abstract_text
        )
        print(abstract_preview)
    else:
        print("\nNo abstract available.")

    print("=" * 80)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Fetch PubMed article metadata",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  %(prog)s 31978945
  %(prog)s 31978945 33515491 25760099
  %(prog)s 31978945 --api-key YOUR_KEY --email you@example.com
        """,
    )

    parser.add_argument("pmids", nargs="+", help="One or more PMIDs to fetch")
    parser.add_argument("--api-key", help="NCBI API key for higher rate limits")
    parser.add_argument("--email", help="Your email address for NCBI")
    parser.add_argument("--tool", default="pubmed-client", help="Tool name for NCBI")

    args = parser.parse_args()

    # Create client with configuration
    if args.api_key or args.email:
        config = pubmed_client.ClientConfig()
        if args.api_key:
            config = config.with_api_key(args.api_key)
        if args.email:
            config = config.with_email(args.email)
        config = config.with_tool(args.tool)
        client = pubmed_client.PubMedClient.with_config(config)
    else:
        client = pubmed_client.PubMedClient()

    # Fetch and display articles
    for pmid in args.pmids:
        print(f"\nFetching metadata for PMID: {pmid}\n")
        article = client.fetch_article(pmid)
        display_article(article)

    # Display summary if multiple articles
    if len(args.pmids) > 1:
        print(f"\n\nSummary: Fetched {len(args.pmids)} articles")


if __name__ == "__main__":
    main()
