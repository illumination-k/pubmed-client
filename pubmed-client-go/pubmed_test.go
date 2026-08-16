package pubmedclient

import (
	"context"
	"errors"
	"net/http"
	"strings"
	"testing"
)

func TestFetchSummariesAgainstStub(t *testing.T) {
	client := newStubClient(t, defaultStub())

	summaries, err := client.FetchSummaries(context.Background(), []string{"31978945"})
	if err != nil {
		t.Fatalf("FetchSummaries failed: %v", err)
	}
	if len(summaries) != 1 {
		t.Fatalf("got %d summaries, want 1", len(summaries))
	}

	summary := summaries[0]
	if summary.PMID != "31978945" {
		t.Errorf("PMID = %q", summary.PMID)
	}
	if summary.Title != "A test article about CRISPR." {
		t.Errorf("Title = %q", summary.Title)
	}
	if summary.Journal != "Nature" {
		t.Errorf("Journal = %q, want %q", summary.Journal, "Nature")
	}
	if summary.FullJournalName != "Nature" {
		t.Errorf("FullJournalName = %q", summary.FullJournalName)
	}
	// ESummary gives formatted names rather than the structured Author the
	// EFetch path returns.
	if len(summary.Authors) != 2 || summary.Authors[0] != "Doe J" {
		t.Errorf("Authors = %v, want [Doe J Roe R]", summary.Authors)
	}
	if summary.DOI != "10.1038/s41586-020-0000-0" {
		t.Errorf("DOI = %q", summary.DOI)
	}
	if summary.PMCID != "PMC7092803" {
		t.Errorf("PMCID = %q", summary.PMCID)
	}
	if summary.EpubDate != "2020 Jan 24" {
		t.Errorf("EpubDate = %q", summary.EpubDate)
	}
	if summary.PMCRefCount != 42 {
		t.Errorf("PMCRefCount = %d, want 42", summary.PMCRefCount)
	}
	if summary.SortPubDate != "2020/02/20 00:00" {
		t.Errorf("SortPubDate = %q", summary.SortPubDate)
	}
	if len(summary.PubTypes) != 2 {
		t.Errorf("PubTypes = %v, want two entries", summary.PubTypes)
	}
	if summary.RecordStatus == "" {
		t.Error("RecordStatus is empty")
	}
}

func TestFetchSummaryReturnsTheFirstRecord(t *testing.T) {
	client := newStubClient(t, defaultStub())

	summary, err := client.FetchSummary(context.Background(), "31978945")
	if err != nil {
		t.Fatalf("FetchSummary failed: %v", err)
	}
	if summary.PMID != "31978945" {
		t.Errorf("PMID = %q", summary.PMID)
	}
}

func TestSearchAndFetchSummariesAgainstStub(t *testing.T) {
	client := newStubClient(t, defaultStub())

	summaries, err := client.SearchAndFetchSummaries(context.Background(), "CRISPR", 5)
	if err != nil {
		t.Fatalf("SearchAndFetchSummaries failed: %v", err)
	}
	if len(summaries) != 1 {
		t.Fatalf("got %d summaries, want 1 (the stub returns one record)", len(summaries))
	}
}

func TestFetchAllByPMIDsAgainstStub(t *testing.T) {
	client := newStubClient(t, defaultStub())

	articles, err := client.FetchAllByPMIDs(context.Background(), []string{"31978945"})
	if err != nil {
		t.Fatalf("FetchAllByPMIDs failed: %v", err)
	}
	if len(articles) != 1 {
		t.Fatalf("got %d articles, want 1", len(articles))
	}
	if articles[0].PMID != "31978945" {
		t.Errorf("PMID = %q", articles[0].PMID)
	}
}

func TestSearchWithFullTextPairsArticlesWithTheirFullText(t *testing.T) {
	client := newStubClient(t, defaultStub())

	results, err := client.SearchWithFullText(context.Background(), "CRISPR", 2)
	if err != nil {
		t.Fatalf("SearchWithFullText failed: %v", err)
	}
	if len(results) != 1 {
		t.Fatalf("got %d results, want 1 (the stub returns one record)", len(results))
	}

	if results[0].Article.PMID != "31978945" {
		t.Errorf("Article.PMID = %q", results[0].Article.PMID)
	}
	// The stub's ELink payload advertises PMC full text, so it must be attached.
	// The PMCID is the one the availability check resolved, not whatever the
	// fetched XML happens to carry.
	if results[0].FullText == nil {
		t.Fatal("FullText is nil, want the stub's PMC article")
	}
	if results[0].FullText.PMCID != "PMC7092803" {
		t.Errorf("FullText.PMCID = %q, want %q", results[0].FullText.PMCID, "PMC7092803")
	}
	if results[0].FullText.Title != "A full-text article for the Go tests" {
		t.Errorf("FullText.Title = %q", results[0].FullText.Title)
	}
}

// --- ELink -------------------------------------------------------------------

func TestGetRelatedArticlesAgainstStub(t *testing.T) {
	client := newStubClient(t, defaultStub())

	related, err := client.GetRelatedArticles(context.Background(), []uint32{31978945})
	if err != nil {
		t.Fatalf("GetRelatedArticles failed: %v", err)
	}
	if len(related.SourcePMIDs) != 1 || related.SourcePMIDs[0] != 31978945 {
		t.Errorf("SourcePMIDs = %v", related.SourcePMIDs)
	}
	if len(related.RelatedPMIDs) != 2 {
		t.Errorf("RelatedPMIDs = %v, want two entries", related.RelatedPMIDs)
	}
	if related.LinkType == "" {
		t.Error("LinkType is empty")
	}
}

func TestGetPMCLinksAgainstStub(t *testing.T) {
	client := newStubClient(t, defaultStub())

	links, err := client.GetPMCLinks(context.Background(), []uint32{31978945})
	if err != nil {
		t.Fatalf("GetPMCLinks failed: %v", err)
	}
	if len(links.PMCIDs) != 1 {
		t.Fatalf("PMCIDs = %v, want one entry", links.PMCIDs)
	}
	if !strings.Contains(links.PMCIDs[0], "7092803") {
		t.Errorf("PMCIDs[0] = %q, want the stub's PMC id", links.PMCIDs[0])
	}
}

func TestGetCitationsAgainstStub(t *testing.T) {
	client := newStubClient(t, defaultStub())

	citations, err := client.GetCitations(context.Background(), []uint32{31978945})
	if err != nil {
		t.Fatalf("GetCitations failed: %v", err)
	}
	if len(citations.CitingPMIDs) != 1 || citations.CitingPMIDs[0] != 33515491 {
		t.Errorf("CitingPMIDs = %v, want [33515491]", citations.CitingPMIDs)
	}
}

func TestELinkSendsNumericPMIDs(t *testing.T) {
	stub := defaultStub()
	var recorded string
	stub.observe = func(r *http.Request) {
		if pathHasSuffix(r, "/elink.fcgi") {
			recorded = r.URL.Query().Get("id")
		}
	}
	client := newStubClient(t, stub)

	if _, err := client.GetRelatedArticles(context.Background(), []uint32{1, 2, 3}); err != nil {
		t.Fatalf("GetRelatedArticles failed: %v", err)
	}
	if recorded != "1,2,3" {
		t.Errorf("id parameter = %q, want %q", recorded, "1,2,3")
	}
}

// --- EInfo -------------------------------------------------------------------

func TestGetDatabaseListAgainstStub(t *testing.T) {
	client := newStubClient(t, defaultStub())

	databases, err := client.GetDatabaseList(context.Background())
	if err != nil {
		t.Fatalf("GetDatabaseList failed: %v", err)
	}
	if len(databases) != 3 || databases[0] != "pubmed" {
		t.Errorf("databases = %v, want [pubmed pmc nuccore]", databases)
	}
}

func TestGetDatabaseInfoAgainstStub(t *testing.T) {
	client := newStubClient(t, defaultStub())

	info, err := client.GetDatabaseInfo(context.Background(), "pubmed")
	if err != nil {
		t.Fatalf("GetDatabaseInfo failed: %v", err)
	}

	if info.Name != "pubmed" || info.MenuName != "PubMed" {
		t.Errorf("Name/MenuName = %q/%q", info.Name, info.MenuName)
	}
	if info.Count == nil {
		t.Fatal("Count is nil, want the stub's record count")
	}
	if *info.Count != 36000000 {
		t.Errorf("Count = %d", *info.Count)
	}
	if len(info.Fields) != 2 {
		t.Fatalf("got %d fields, want 2", len(info.Fields))
	}
	// The Y/N flags NCBI sends must arrive as booleans.
	if info.Fields[0].IsDate {
		t.Error("Fields[0].IsDate = true, want false")
	}
	if !info.Fields[1].IsDate {
		t.Error("Fields[1].IsDate = false, want true")
	}
	if info.Fields[0].TermCount == nil || *info.Fields[0].TermCount != 1000 {
		t.Errorf("Fields[0].TermCount = %v", info.Fields[0].TermCount)
	}
	if len(info.Links) != 1 || info.Links[0].TargetDB != "pmc" {
		t.Errorf("Links = %+v", info.Links)
	}
}

// --- ESpell, EGQuery, ECitMatch ----------------------------------------------

func TestSpellCheckAgainstStub(t *testing.T) {
	client := newStubClient(t, defaultStub())

	result, err := client.SpellCheck(context.Background(), "asthmaa treetment")
	if err != nil {
		t.Fatalf("SpellCheck failed: %v", err)
	}

	if result.CorrectedQuery != "asthma treatment" {
		t.Errorf("CorrectedQuery = %q", result.CorrectedQuery)
	}
	if !result.HasCorrections() {
		t.Error("HasCorrections() = false, want true")
	}

	replacements := result.Replacements()
	if len(replacements) != 2 || replacements[0] != "asthma" || replacements[1] != "treatment" {
		t.Errorf("Replacements() = %v, want [asthma treatment]", replacements)
	}
	// The segment list interleaves changed and unchanged pieces; the unchanged
	// separator must survive with Replaced false.
	if len(result.SpelledQuery) != 3 {
		t.Fatalf("got %d segments, want 3", len(result.SpelledQuery))
	}
	if result.SpelledQuery[1].Replaced {
		t.Error("SpelledQuery[1].Replaced = true, want false for the separator")
	}
}

func TestSpellCheckDBSendsTheDatabase(t *testing.T) {
	stub := defaultStub()
	var recorded string
	stub.observe = func(r *http.Request) {
		if pathHasSuffix(r, "/espell.fcgi") {
			recorded = r.URL.Query().Get("db")
		}
	}
	client := newStubClient(t, stub)

	if _, err := client.SpellCheckDB(context.Background(), "asthmaa", "pmc"); err != nil {
		t.Fatalf("SpellCheckDB failed: %v", err)
	}
	if recorded != "pmc" {
		t.Errorf("db parameter = %q, want %q", recorded, "pmc")
	}
}

func TestSpellCheckDBRejectsAnEmptyDatabase(t *testing.T) {
	client := newStubClient(t, defaultStub())

	_, err := client.SpellCheckDB(context.Background(), "asthmaa", "")
	if !errors.Is(err, ErrInvalidArgument) {
		t.Fatalf("SpellCheckDB with no database = %v, want ErrInvalidArgument", err)
	}
}

func TestNoCorrectionsIsReportedAsSuch(t *testing.T) {
	result := SpellCheckResult{Query: "asthma", CorrectedQuery: "asthma"}
	if result.HasCorrections() {
		t.Error("HasCorrections() = true for an unchanged query")
	}

	empty := SpellCheckResult{Query: "asthma"}
	if empty.HasCorrections() {
		t.Error("HasCorrections() = true for an empty correction")
	}
	if empty.Replacements() != nil {
		t.Errorf("Replacements() = %v, want nil", empty.Replacements())
	}
}

func TestGlobalQueryAgainstStub(t *testing.T) {
	client := newStubClient(t, defaultStub())

	results, err := client.GlobalQuery(context.Background(), "asthma")
	if err != nil {
		t.Fatalf("GlobalQuery failed: %v", err)
	}

	if results.Term != "asthma" {
		t.Errorf("Term = %q", results.Term)
	}
	if len(results.Results) != 3 {
		t.Fatalf("got %d results, want 3", len(results.Results))
	}
	// NonZero drops the database with no matches.
	if got := results.NonZero(); len(got) != 2 {
		t.Errorf("NonZero() returned %d entries, want 2", len(got))
	}

	count, ok := results.CountFor("pmc")
	if !ok || count != 89012 {
		t.Errorf("CountFor(pmc) = (%d, %v), want (89012, true)", count, ok)
	}
	if _, ok := results.CountFor("nowhere"); ok {
		t.Error("CountFor(nowhere) reported a hit")
	}
}

func TestMatchCitationsAgainstStub(t *testing.T) {
	client := newStubClient(t, defaultStub())

	matches, err := client.MatchCitations(context.Background(), []CitationQuery{
		{
			Journal:    "proc natl acad sci u s a",
			Year:       "1991",
			Volume:     "88",
			FirstPage:  "3248",
			AuthorName: "mann bj",
			Key:        "Art1",
		},
	})
	if err != nil {
		t.Fatalf("MatchCitations failed: %v", err)
	}

	// The stub answers with one of each status, so every branch of the status
	// mapping is covered.
	if len(matches.Matches) != 3 {
		t.Fatalf("got %d matches, want 3", len(matches.Matches))
	}

	found := matches.Found()
	if len(found) != 1 {
		t.Fatalf("Found() returned %d matches, want 1", len(found))
	}
	if found[0].PMID != "2014248" {
		t.Errorf("Found()[0].PMID = %q", found[0].PMID)
	}
	if found[0].Key != "Art1" {
		t.Errorf("Found()[0].Key = %q, want the key from the query", found[0].Key)
	}

	statuses := []CitationMatchStatus{CitationFound, CitationNotFound, CitationAmbiguous}
	for i, want := range statuses {
		if matches.Matches[i].Status != want {
			t.Errorf("Matches[%d].Status = %q, want %q", i, matches.Matches[i].Status, want)
		}
	}
}
