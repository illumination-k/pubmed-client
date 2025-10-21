"""
Example: Searching PubMed

Search PubMed for articles matching a query and display results.

Usage:
    python 03_search_pubmed.py "COVID-19 vaccine"
    python 03_search_pubmed.py "CRISPR gene editing" --limit 20
    python 03_search_pubmed.py "machine learning" --api-key YOUR_KEY
"""

import argparse

import pubmed_client


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Search PubMed for articles",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  %(prog)s "COVID-19 vaccine"
  %(prog)s "CRISPR gene editing" --limit 20
  %(prog)s "machine learning bioinformatics" --api-key YOUR_KEY
        """,
    )

    parser.add_argument("query", help="Search query for PubMed")
    parser.add_argument(
        "--limit",
        type=int,
        default=10,
        metavar="N",
        help="Number of articles to retrieve (default: 10)",
    )
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

    print(f"Searching for: '{args.query}' (limit: {args.limit})\n")
    print("=" * 80)

    # Search and fetch articles
    articles = client.search_and_fetch(args.query, args.limit)

    print(f"\nFound {len(articles)} articles:\n")

    # Display results
    for i, article in enumerate(articles, start=1):
        print(f"{i}. [{article.pmid}] {article.title}")
        print(f"   Journal: {article.journal} ({article.pub_date})")
        print(f"   Authors: {article.author_count}")

        if article.doi:
            print(f"   DOI: {article.doi}")

        # Show abstract preview
        if article.abstract_text:
            abstract_preview = (
                article.abstract_text[:150] + "..."
                if len(article.abstract_text) > 150
                else article.abstract_text
            )
            print(f"   Abstract: {abstract_preview}")

        print()

    # Display summary
    print("=" * 80)
    print(f"\nSummary:")
    print(f"  Total articles: {len(articles)}")
    print(f"  With DOI: {sum(1 for a in articles if a.doi)}")
    print(f"  With abstract: {sum(1 for a in articles if a.abstract_text)}")


if __name__ == "__main__":
    main()
