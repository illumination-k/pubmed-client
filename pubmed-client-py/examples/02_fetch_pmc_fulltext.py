"""
Example: Retrieving PMC Full-Text Article Metadata from PMCID

This example demonstrates how to fetch full-text article data from PubMed Central (PMC)
using a PMCID. It shows how to access article sections, figures, tables, references,
and author information.

Usage:
    python 02_fetch_pmc_fulltext.py PMC7906746
    python 02_fetch_pmc_fulltext.py PMC7906746 PMC10618641
    python 02_fetch_pmc_fulltext.py PMC7906746 --api-key YOUR_KEY
"""

import argparse

import pubmed_client


def display_pmc_article(article: pubmed_client.PmcFullText) -> None:
    """Display PMC full-text article in a formatted way"""
    print("=" * 80)
    print(f"Title: {article.title}")
    print(f"PMCID: {article.pmcid}")

    if article.pmid:
        print(f"PMID: {article.pmid}")

    if article.doi:
        print(f"DOI: {article.doi}")

    # Display authors
    authors = article.authors()
    print(f"\nAuthors ({len(authors)}):")
    for i, author in enumerate(authors[:10], start=1):  # First 10 authors
        author_info = f"{i}. {author.full_name}"

        if author.orcid:
            author_info += f" (ORCID: {author.orcid})"
        if author.email:
            author_info += f" <{author.email}>"
        if author.is_corresponding:
            author_info += " [Corresponding]"

        print(f"  {author_info}")

    if len(authors) > 10:
        print(f"  ... and {len(authors) - 10} more authors")

    # Display article sections
    sections = article.sections()
    print(f"\nArticle Sections ({len(sections)}):")
    for i, section in enumerate(sections[:5], start=1):  # First 5 sections
        section_title = section.title if section.title else f"Section {i}"
        content_length = len(section.content)
        section_type = f" [{section.section_type}]" if section.section_type else ""

        print(f"  {i}. {section_title}{section_type} - {content_length} characters")

    if len(sections) > 5:
        print(f"  ... and {len(sections) - 5} more sections")

    # Display figures
    figures = article.figures()
    print(f"\nFigures ({len(figures)}):")
    if figures:
        for i, figure in enumerate(figures[:3], start=1):  # First 3 figures
            print(f"  {i}. {figure.label if figure.label else figure.id}")
            if figure.caption:
                caption_preview = (
                    figure.caption[:100] + "..." if len(figure.caption) > 100 else figure.caption
                )
                print(f"     Caption: {caption_preview}")
        if len(figures) > 3:
            print(f"  ... and {len(figures) - 3} more figures")
    else:
        print("  No figures available.")

    # Display tables
    tables = article.tables()
    print(f"\nTables ({len(tables)}):")
    if tables:
        for i, table in enumerate(tables[:3], start=1):  # First 3 tables
            print(f"  {i}. {table.label if table.label else table.id}")
        if len(tables) > 3:
            print(f"  ... and {len(tables) - 3} more tables")
    else:
        print("  No tables available.")

    # Display references
    references = article.references()
    print(f"\nReferences: {len(references)} total")

    print("=" * 80)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Fetch PMC full-text article metadata",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  %(prog)s PMC7906746
  %(prog)s PMC7906746 PMC10618641
  %(prog)s PMC7906746 --api-key YOUR_KEY --email you@example.com
        """,
    )

    parser.add_argument("pmcids", nargs="+", help="One or more PMCIDs to fetch")
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
        client = pubmed_client.PmcClient.with_config(config)
    else:
        client = pubmed_client.PmcClient()

    # Fetch and display articles
    for pmcid in args.pmcids:
        print(f"\nFetching full-text metadata for PMCID: {pmcid}\n")
        article = client.fetch_full_text(pmcid)
        display_pmc_article(article)

    # Display summary if multiple articles
    if len(args.pmcids) > 1:
        print(f"\n\nSummary: Fetched {len(args.pmcids)} PMC articles")


if __name__ == "__main__":
    main()
