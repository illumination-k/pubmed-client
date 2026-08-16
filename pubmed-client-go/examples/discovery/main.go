// Command discovery demonstrates the parts of the bindings beyond plain
// search: the query builder, ESummary, and the ELink/EInfo/ESpell discovery
// APIs.
//
// Build the Rust archive first (see ../../README.md), then:
//
//	go run ./examples/discovery
package main

import (
	"context"
	"fmt"
	"log"
	"strconv"
	"strings"
	"time"

	pubmedclient "github.com/illumination-k/pubmed-client/pubmed-client-go"
)

func main() {
	client, err := pubmedclient.New(&pubmedclient.Config{
		Email: "you@example.com",
		Tool:  "pubmed-client-go-example",
	})
	if err != nil {
		log.Fatalf("failed to create client: %v", err)
	}
	defer client.Close()

	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Minute)
	defer cancel()

	// The builder records what you ask for and lets Rust assemble the field
	// tags, so the query below is the same string every binding would produce.
	query := pubmedclient.NewSearchQuery().
		TitleOrAbstract("CRISPR").
		MeshTerm("Gene Editing").
		PublishedAfter(pubmedclient.Year(2020)).
		ArticleType("Review").
		HumanStudiesOnly().
		Limit(5).
		Sort(pubmedclient.SortPublicationDate)

	if err := query.Validate(); err != nil {
		log.Fatalf("query is not well formed: %v", err)
	}
	fmt.Printf("query: %s\n\n", query)

	// Summaries are much cheaper than full metadata when only titles, journals
	// and dates are needed.
	pmids, err := client.Search(ctx, query)
	if err != nil {
		log.Fatalf("search failed: %v", err)
	}

	summaries, err := client.FetchSummaries(ctx, pmids)
	if err != nil {
		log.Fatalf("failed to fetch summaries: %v", err)
	}
	for _, summary := range summaries {
		fmt.Printf("%s  %s\n    %s (%s), cited by %d in PMC\n",
			summary.PMID, summary.Title, summary.Journal, summary.PubDate, summary.PMCRefCount)
	}
	fmt.Println()

	if len(pmids) == 0 {
		return
	}

	// The ELink APIs take numeric PMIDs, matching NCBI's own UID parameter.
	uids, err := toUIDs(pmids[:1])
	if err != nil {
		log.Fatalf("unexpected PMID format: %v", err)
	}

	related, err := client.GetRelatedArticles(ctx, uids)
	if err != nil {
		log.Fatalf("failed to find related articles: %v", err)
	}
	fmt.Printf("PMID %d has %d related articles\n", uids[0], len(related.RelatedPMIDs))

	citations, err := client.GetCitations(ctx, uids)
	if err != nil {
		log.Fatalf("failed to find citations: %v", err)
	}
	fmt.Printf("PMID %d is cited by %d PMC articles\n\n", uids[0], len(citations.CitingPMIDs))

	// ESpell catches typos before they turn into an empty result set.
	spelling, err := client.SpellCheck(ctx, "asthmaa treetment")
	if err != nil {
		log.Fatalf("spell check failed: %v", err)
	}
	if spelling.HasCorrections() {
		fmt.Printf("did you mean %q? (corrected: %s)\n\n",
			spelling.CorrectedQuery, strings.Join(spelling.Replacements(), ", "))
	}

	// EInfo describes what a database indexes, including every searchable tag.
	info, err := client.GetDatabaseInfo(ctx, "pubmed")
	if err != nil {
		log.Fatalf("failed to describe pubmed: %v", err)
	}
	fmt.Printf("%s: %d searchable fields", info.MenuName, len(info.Fields))
	if info.Count != nil {
		fmt.Printf(", %d records", *info.Count)
	}
	fmt.Println()
}

// toUIDs converts PMID strings to the numeric UIDs the ELink APIs take.
func toUIDs(pmids []string) ([]uint32, error) {
	uids := make([]uint32, 0, len(pmids))
	for _, pmid := range pmids {
		parsed, err := strconv.ParseUint(pmid, 10, 32)
		if err != nil {
			return nil, fmt.Errorf("PMID %q is not numeric: %w", pmid, err)
		}
		uids = append(uids, uint32(parsed))
	}
	return uids, nil
}
