//go:build integration

// Live NCBI API tests. They are excluded from the default build by the
// `integration` tag and additionally skip unless PUBMED_REAL_API_TESTS=1, the
// same opt-in the Rust crate and the R bindings use.
//
//	make test-integration
package pubmedclient

import (
	"os"
	"strings"
	"testing"
	"time"
)

// A stable, long-standing open-access record used as the PMC fixture.
const (
	livePMID  = "31978945"
	livePMCID = "PMC7906746"
)

func liveClient(t *testing.T) *Client {
	t.Helper()

	if os.Getenv("PUBMED_REAL_API_TESTS") != "1" {
		t.Skip("set PUBMED_REAL_API_TESTS=1 to run tests against the live NCBI API")
	}

	client, err := New(&Config{
		APIKey:  os.Getenv("NCBI_API_KEY"),
		Email:   os.Getenv("PUBMED_EMAIL"),
		Tool:    "pubmed-client-go-integration-tests",
		Timeout: 60 * time.Second,
	})
	if err != nil {
		t.Fatalf("New failed: %v", err)
	}
	t.Cleanup(func() { _ = client.Close() })

	return client
}

func TestLiveSearchArticles(t *testing.T) {
	client := liveClient(t)

	pmids, err := client.SearchArticles("CRISPR gene editing", 5)
	if err != nil {
		t.Fatalf("SearchArticles failed: %v", err)
	}
	if len(pmids) == 0 {
		t.Fatal("SearchArticles returned no PMIDs")
	}
	if len(pmids) > 5 {
		t.Errorf("got %d PMIDs, want at most 5", len(pmids))
	}
}

func TestLiveFetchArticle(t *testing.T) {
	client := liveClient(t)

	article, err := client.FetchArticle(livePMID)
	if err != nil {
		t.Fatalf("FetchArticle failed: %v", err)
	}
	if article.PMID != livePMID {
		t.Errorf("PMID = %q, want %q", article.PMID, livePMID)
	}
	if article.Title == "" {
		t.Error("Title is empty")
	}
	if article.Journal == "" {
		t.Error("Journal is empty")
	}
}

func TestLiveFetchArticles(t *testing.T) {
	client := liveClient(t)

	articles, err := client.FetchArticles([]string{livePMID, "33515491"})
	if err != nil {
		t.Fatalf("FetchArticles failed: %v", err)
	}
	if len(articles) == 0 {
		t.Fatal("FetchArticles returned nothing")
	}
	for i, article := range articles {
		if article.PMID == "" {
			t.Errorf("articles[%d] has an empty PMID", i)
		}
	}
}

func TestLiveSearchAndFetch(t *testing.T) {
	client := liveClient(t)

	articles, err := client.SearchAndFetch("machine learning radiology", 3)
	if err != nil {
		t.Fatalf("SearchAndFetch failed: %v", err)
	}
	if len(articles) == 0 {
		t.Fatal("SearchAndFetch returned nothing")
	}
	if articles[0].Title == "" {
		t.Error("first article has an empty Title")
	}
}

func TestLiveFetchFullText(t *testing.T) {
	client := liveClient(t)

	article, err := client.FetchFullText(livePMCID)
	if err != nil {
		t.Fatalf("FetchFullText failed: %v", err)
	}
	if !strings.Contains(article.PMCID, "7906746") {
		t.Errorf("PMCID = %q, want it to contain 7906746", article.PMCID)
	}
	if article.Title == "" {
		t.Error("Title is empty")
	}
	if len(article.Sections) == 0 {
		t.Error("Sections is empty")
	}
}

func TestLiveFetchMarkdown(t *testing.T) {
	client := liveClient(t)

	markdown, err := client.FetchMarkdown(livePMCID)
	if err != nil {
		t.Fatalf("FetchMarkdown failed: %v", err)
	}
	if markdown == "" {
		t.Fatal("FetchMarkdown returned an empty document")
	}
	if !strings.Contains(markdown, "#") {
		t.Error("Markdown output contains no headings")
	}
}

func TestLiveCheckPMCAvailability(t *testing.T) {
	client := liveClient(t)

	pmcid, available, err := client.CheckPMCAvailability(livePMID)
	if err != nil {
		t.Fatalf("CheckPMCAvailability failed: %v", err)
	}
	if available && pmcid == "" {
		t.Error("reported as available but the PMCID is empty")
	}
	if !available && pmcid != "" {
		t.Errorf("reported as unavailable but the PMCID is %q", pmcid)
	}
}

// A PMID that does not exist must produce an error rather than an empty result.
func TestLiveInvalidPMID(t *testing.T) {
	client := liveClient(t)

	if _, err := client.FetchArticle("not-a-pmid"); err == nil {
		t.Error("FetchArticle with an invalid PMID succeeded, want error")
	}
}
