// Command basic demonstrates the pubmed-client Go bindings: search PubMed,
// print article metadata, and render a PMC article to Markdown.
//
// Build the Rust archive first (see ../../README.md), then:
//
//	go run ./examples/basic
package main

import (
	"fmt"
	"log"
	"strings"

	pubmedclient "github.com/illumination-k/pubmed-client/pubmed-client-go"
)

func main() {
	// NCBI asks callers to identify themselves. An API key (Config.APIKey)
	// raises the rate limit from 3 to 10 requests per second.
	client, err := pubmedclient.New(&pubmedclient.Config{
		Email: "you@example.com",
		Tool:  "pubmed-client-go-example",
	})
	if err != nil {
		log.Fatalf("failed to create client: %v", err)
	}
	defer client.Close()

	fmt.Printf("pubmed-client %s\n\n", pubmedclient.Version())

	articles, err := client.SearchAndFetch("CRISPR gene editing", 3)
	if err != nil {
		log.Fatalf("search failed: %v", err)
	}

	for _, article := range articles {
		fmt.Printf("PMID %s\n", article.PMID)
		fmt.Printf("  %s\n", article.Title)
		fmt.Printf("  %s (%s)\n", article.Journal, article.PubDate)
		if len(article.Authors) > 0 {
			names := make([]string, 0, len(article.Authors))
			for _, author := range article.Authors {
				names = append(names, author.FullName)
			}
			fmt.Printf("  %s\n", strings.Join(names, ", "))
		}
		if article.DOI != "" {
			fmt.Printf("  https://doi.org/%s\n", article.DOI)
		}
		fmt.Println()
	}

	// Full text is only available for the PMC Open Access subset.
	const pmcid = "PMC7906746"

	fullText, err := client.FetchFullText(pmcid)
	if err != nil {
		log.Fatalf("failed to fetch full text for %s: %v", pmcid, err)
	}
	fmt.Printf("%s: %d sections, %d references, %d figures\n",
		fullText.PMCID, len(fullText.Sections), len(fullText.References), fullText.FigureCount)

	markdown, err := client.FetchMarkdown(pmcid)
	if err != nil {
		log.Fatalf("failed to render %s: %v", pmcid, err)
	}
	fmt.Printf("\n--- Markdown (first 400 chars) ---\n%s\n", truncate(markdown, 400))
}

func truncate(text string, limit int) string {
	if len(text) <= limit {
		return text
	}
	return text[:limit] + "..."
}
