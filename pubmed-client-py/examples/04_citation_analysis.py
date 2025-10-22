"""
Example: Citation Analysis - Count Citations for PMIDs

Analyze citations for PubMed articles and compare citation counts.

NOTE: Citation counts are from PubMed database only (peer-reviewed articles).
This may be lower than Google Scholar or scite.ai which include preprints,
books, and conference proceedings. This is expected behavior.

Usage:
    python 04_citation_analysis.py 31978945
    python 04_citation_analysis.py 31978945 25760099 33515491
    python 04_citation_analysis.py 31978945 --show-citing 10
"""

import argparse

import pubmed_client


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Analyze citations for PubMed articles",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  %(prog)s 31978945
  %(prog)s 31978945 25760099 33515491
  %(prog)s 31978945 --show-citing 10
  %(prog)s 31978945 --api-key YOUR_KEY
        """,
    )

    parser.add_argument("pmids", type=int, nargs="+", help="One or more PMIDs to analyze")
    parser.add_argument(
        "--show-citing",
        type=int,
        default=5,
        metavar="N",
        help="Number of citing articles to show (default: 5)",
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

    print("Citation Analysis")
    print("=" * 80)

    results: list[dict[str, int | str]] = []

    # Analyze each PMID
    for pmid in args.pmids:
        print(f"\nAnalyzing PMID: {pmid}")

        # Get article metadata
        article = client.fetch_article(str(pmid))

        # Get citations
        citations = client.get_citations([pmid])
        citation_count = len(citations)

        print(f"  Title: {article.title[:70]}...")
        print(f"  Journal: {article.journal} ({article.pub_date})")
        print(f"  Citation count: {citation_count}")

        # Show citing articles
        if citations.citing_pmids and args.show_citing > 0:
            show_count = min(args.show_citing, len(citations.citing_pmids))
            print(f"\n  First {show_count} citing articles (PMIDs):")
            for citing_pmid in citations.citing_pmids[:show_count]:
                print(f"    - {citing_pmid}")

            if len(citations.citing_pmids) > show_count:
                print(f"    ... and {len(citations.citing_pmids) - show_count} more")

        results.append(
            {
                "pmid": pmid,
                "title": article.title,
                "pub_date": article.pub_date,
                "journal": article.journal,
                "citation_count": citation_count,
            }
        )

    # Display comparison if multiple PMIDs
    if len(args.pmids) > 1:
        print("\n" + "=" * 80)
        print("Citation Count Comparison:")
        print("-" * 80)

        # Sort by citation count
        results.sort(key=lambda x: int(x["citation_count"]), reverse=True)

        for i, result in enumerate(results, start=1):
            title = result["title"]
            title_str = str(title)[:60] if isinstance(title, str) else str(title)
            print(f"\n{i}. PMID: {result['pmid']} - Citations: {result['citation_count']}")
            print(f"   Title: {title_str}...")
            print(f"   Published: {result['pub_date']}")

        # Display statistics
        citation_counts = [int(r["citation_count"]) for r in results]
        total_citations = sum(citation_counts)
        avg_citations = total_citations / len(citation_counts)
        max_citations = max(citation_counts)
        min_citations = min(citation_counts)

        print("\n" + "-" * 80)
        print("Statistics:")
        print(f"  Total citations: {total_citations}")
        print(f"  Average citations per paper: {avg_citations:.2f}")
        print(f"  Maximum citations: {max_citations}")
        print(f"  Minimum citations: {min_citations}")

    print("\n" + "=" * 80)


if __name__ == "__main__":
    main()
