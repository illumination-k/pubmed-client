//go:build integration

// Live NCBI API tests. They are excluded from the default build by the
// `integration` tag and additionally skip unless PUBMED_REAL_API_TESTS=1, the
// same opt-in the Rust crate and the R bindings use.
//
//	MISE_ENV=go mise run go:test-integration
package pubmedclient

import (
	"context"
	"errors"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"testing"
	"time"
)

// A stable, long-standing open-access record used as the PMC fixture.
const (
	livePMID  = "31978945"
	livePMCID = "PMC7906746"
)

func liveClient(t *testing.T) (*Client, context.Context) {
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

	// Bound each test independently, so one hanging call cannot stall the run.
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Minute)
	t.Cleanup(cancel)

	return client, ctx
}

func TestLiveSearchArticles(t *testing.T) {
	client, ctx := liveClient(t)

	pmids, err := client.SearchArticles(ctx, "CRISPR gene editing", 5)
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

func TestLiveSearchWithQueryBuilder(t *testing.T) {
	client, ctx := liveClient(t)

	query := NewSearchQuery().
		TitleOrAbstract("CRISPR").
		PublishedAfter(Year(2020)).
		ArticleType("Review").
		Limit(3).
		Sort(SortPublicationDate)

	if err := query.Validate(); err != nil {
		t.Fatalf("Validate failed: %v", err)
	}

	pmids, err := client.Search(ctx, query)
	if err != nil {
		t.Fatalf("Search failed: %v", err)
	}
	if len(pmids) == 0 {
		t.Fatal("Search returned no PMIDs")
	}
	if len(pmids) > 3 {
		t.Errorf("got %d PMIDs, want at most 3 (the builder's limit)", len(pmids))
	}
}

func TestLiveFetchArticle(t *testing.T) {
	client, ctx := liveClient(t)

	article, err := client.FetchArticle(ctx, livePMID)
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
	client, ctx := liveClient(t)

	articles, err := client.FetchArticles(ctx, []string{livePMID, "33515491"})
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
	client, ctx := liveClient(t)

	articles, err := client.SearchAndFetch(ctx, "machine learning radiology", 3)
	if err != nil {
		t.Fatalf("SearchAndFetch failed: %v", err)
	}
	if len(articles) == 0 {
		t.Fatal("SearchAndFetch returned nothing")
	}
	if articles[0].Title == "" {
		t.Error("first article has an empty Title")
	}

	// Round-trip through the exporter: whatever the live API produced must be
	// re-encodable into something the Rust formatter accepts.
	bibtex, err := ExportArticles(articles, FormatBibTeX)
	if err != nil {
		t.Fatalf("ExportArticles failed: %v", err)
	}
	if !strings.Contains(bibtex, "@article{") {
		t.Errorf("BibTeX output looks wrong:\n%s", bibtex)
	}
}

func TestLiveFetchSummaries(t *testing.T) {
	client, ctx := liveClient(t)

	summaries, err := client.FetchSummaries(ctx, []string{livePMID})
	if err != nil {
		t.Fatalf("FetchSummaries failed: %v", err)
	}
	if len(summaries) == 0 {
		t.Fatal("FetchSummaries returned nothing")
	}
	if summaries[0].PMID != livePMID {
		t.Errorf("PMID = %q, want %q", summaries[0].PMID, livePMID)
	}
	if summaries[0].Title == "" {
		t.Error("Title is empty")
	}
}

func TestLiveELink(t *testing.T) {
	client, ctx := liveClient(t)

	uid, err := strconv.ParseUint(livePMID, 10, 32)
	if err != nil {
		t.Fatalf("the fixture PMID is not numeric: %v", err)
	}
	uids := []uint32{uint32(uid)}

	related, err := client.GetRelatedArticles(ctx, uids)
	if err != nil {
		t.Fatalf("GetRelatedArticles failed: %v", err)
	}
	if len(related.RelatedPMIDs) == 0 {
		t.Error("GetRelatedArticles found nothing for a well-cited article")
	}

	links, err := client.GetPMCLinks(ctx, uids)
	if err != nil {
		t.Fatalf("GetPMCLinks failed: %v", err)
	}
	if len(links.PMCIDs) == 0 {
		t.Error("GetPMCLinks found no PMC full text for an open-access article")
	}

	if _, err := client.GetCitations(ctx, uids); err != nil {
		t.Fatalf("GetCitations failed: %v", err)
	}
}

func TestLiveEInfo(t *testing.T) {
	client, ctx := liveClient(t)

	databases, err := client.GetDatabaseList(ctx)
	if err != nil {
		t.Fatalf("GetDatabaseList failed: %v", err)
	}
	if len(databases) == 0 {
		t.Fatal("GetDatabaseList returned nothing")
	}

	info, err := client.GetDatabaseInfo(ctx, "pubmed")
	if err != nil {
		t.Fatalf("GetDatabaseInfo failed: %v", err)
	}
	if info.Name != "pubmed" {
		t.Errorf("Name = %q, want %q", info.Name, "pubmed")
	}
	if len(info.Fields) == 0 {
		t.Error("pubmed reported no searchable fields")
	}
}

func TestLiveSpellCheck(t *testing.T) {
	client, ctx := liveClient(t)

	result, err := client.SpellCheck(ctx, "asthmaa")
	if err != nil {
		t.Fatalf("SpellCheck failed: %v", err)
	}
	if !result.HasCorrections() {
		t.Errorf("no correction suggested for %q (corrected: %q)", result.Query, result.CorrectedQuery)
	}
}

func TestLiveGlobalQuery(t *testing.T) {
	client, ctx := liveClient(t)

	results, err := client.GlobalQuery(ctx, "asthma")
	if err != nil {
		t.Fatalf("GlobalQuery failed: %v", err)
	}
	if count, ok := results.CountFor("pubmed"); !ok || count == 0 {
		t.Errorf("CountFor(pubmed) = (%d, %v), want a non-zero count", count, ok)
	}
}

func TestLiveMatchCitations(t *testing.T) {
	client, ctx := liveClient(t)

	matches, err := client.MatchCitations(ctx, []CitationQuery{{
		Journal:    "proc natl acad sci u s a",
		Year:       "1991",
		Volume:     "88",
		FirstPage:  "3248",
		AuthorName: "mann bj",
		Key:        "Art1",
	}})
	if err != nil {
		t.Fatalf("MatchCitations failed: %v", err)
	}
	if len(matches.Matches) == 0 {
		t.Fatal("MatchCitations returned nothing")
	}
	if matches.Matches[0].Key != "Art1" {
		t.Errorf("Key = %q, want the key from the query", matches.Matches[0].Key)
	}
}

func TestLiveFetchFullText(t *testing.T) {
	client, ctx := liveClient(t)

	article, err := client.FetchFullText(ctx, livePMCID)
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

func TestLiveFetchXML(t *testing.T) {
	client, ctx := liveClient(t)

	xml, err := client.FetchXML(ctx, livePMCID)
	if err != nil {
		t.Fatalf("FetchXML failed: %v", err)
	}
	if !strings.Contains(xml, "<article") {
		t.Errorf("FetchXML did not return JATS XML: %.120q", xml)
	}
}

func TestLiveFetchMarkdown(t *testing.T) {
	client, ctx := liveClient(t)

	markdown, err := client.FetchMarkdown(ctx, livePMCID)
	if err != nil {
		t.Fatalf("FetchMarkdown failed: %v", err)
	}
	if markdown == "" {
		t.Fatal("FetchMarkdown returned an empty document")
	}
	if !strings.Contains(markdown, "#") {
		t.Error("Markdown output contains no headings")
	}

	withFrontmatter, err := client.FetchMarkdownWithOptions(ctx, livePMCID,
		MarkdownOptions{YAMLFrontmatter: Bool(true), IncludeTOC: Bool(true)})
	if err != nil {
		t.Fatalf("FetchMarkdownWithOptions failed: %v", err)
	}
	if !strings.HasPrefix(withFrontmatter, "---") {
		t.Errorf("YAMLFrontmatter did not take effect: %.120q", withFrontmatter)
	}
}

func TestLiveCheckPMCAvailability(t *testing.T) {
	client, ctx := liveClient(t)

	pmcid, available, err := client.CheckPMCAvailability(ctx, livePMID)
	if err != nil {
		t.Fatalf("CheckPMCAvailability failed: %v", err)
	}
	if available && pmcid == "" {
		t.Error("reported as available but the PMCID is empty")
	}
	if !available && pmcid != "" {
		t.Errorf("reported as unavailable but the PMCID is %q", pmcid)
	}
	// Regression guard: the PMCID must be a bare identifier, with no stray
	// quoting from the JSON it was read out of.
	if available && !strings.HasPrefix(pmcid, "PMC") {
		t.Errorf("PMCID = %q, want a PMC-prefixed identifier", pmcid)
	}
	if strings.ContainsAny(pmcid, `"'`) {
		t.Errorf("PMCID = %q, want no quoting", pmcid)
	}
}

func TestLiveIsOASubset(t *testing.T) {
	client, ctx := liveClient(t)

	info, err := client.IsOASubset(ctx, livePMCID)
	if err != nil {
		t.Fatalf("IsOASubset failed: %v", err)
	}
	if !strings.Contains(info.PMCID, "7906746") {
		t.Errorf("PMCID = %q", info.PMCID)
	}
	if !info.IsOASubset {
		t.Skipf("%s is no longer in the OA subset: %s", livePMCID, info.ErrorMessage)
	}
}

func TestLiveExtractFigures(t *testing.T) {
	client, ctx := liveClient(t)

	outputDir := filepath.Join(t.TempDir(), "figures")
	figures, err := client.ExtractFigures(ctx, livePMCID, outputDir)
	if err != nil {
		t.Fatalf("ExtractFigures failed: %v", err)
	}
	if len(figures) == 0 {
		t.Skip("the fixture article has no downloadable figures")
	}

	for i, figure := range figures {
		if figure.Path == "" {
			t.Errorf("figures[%d] has no path", i)
			continue
		}
		if _, err := os.Stat(figure.Path); err != nil {
			t.Errorf("figures[%d] path %q does not exist: %v", i, figure.Path, err)
		}
	}
}

// A PMID that does not exist must produce an error rather than an empty result.
func TestLiveInvalidPMID(t *testing.T) {
	client, ctx := liveClient(t)

	if _, err := client.FetchArticle(ctx, "not-a-pmid"); err == nil {
		t.Error("FetchArticle with an invalid PMID succeeded, want error")
	}
}

// Cancellation has to work against the real API too, not just a local stub.
func TestLiveContextCancellation(t *testing.T) {
	client, ctx := liveClient(t)

	cancelled, cancel := context.WithCancel(ctx)
	cancel()

	if _, err := client.SearchArticles(cancelled, "cancer", 1); !errors.Is(err, context.Canceled) {
		t.Errorf("SearchArticles with a cancelled context = %v, want context.Canceled", err)
	}
}
