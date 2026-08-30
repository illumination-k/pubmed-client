package pubmedclient

import (
	"context"
	"errors"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"
)

// A stub Europe PMC server, so these run offline like the E-utilities stubs in
// stub_test.go. The payloads are trimmed versions of real Europe PMC responses.
//
// Europe PMC lives on a different host from the E-utilities and needs its own
// base URL, which is why Config carries EuropePMCBaseURL alongside BaseURL.

const epmcSearchResponse = `{
  "hitCount": 2,
  "nextCursorMark": "AoJ456",
  "resultList": {"result": [
    {
      "id": "33515491", "source": "MED", "pmid": "33515491", "pmcid": "PMC7894017",
      "doi": "10.1000/x", "title": "A matched article", "authorString": "Smith J.",
      "journalTitle": "Nature", "pubYear": "2021", "isOpenAccess": "Y",
      "citedByCount": 42
    },
    {"id": "PPR12345", "source": "PPR", "title": "A preprint", "pubYear": "2022"}
  ]}
}`

const epmcReferencesResponse = `{
  "hitCount": 2,
  "referenceList": {"reference": [
    {
      "id": 12345, "source": "MED", "citationType": "JOURNAL ARTICLE",
      "title": "Cited work one", "authorString": "Doe J.",
      "journalAbbreviation": "Nature", "pubYear": 2010, "volume": "5",
      "issue": "2", "pageInfo": "100-110", "pmid": "12345", "doi": "10.1/abc"
    },
    {"title": "Unmatched reference", "pubYear": 1999}
  ]}
}`

const epmcCitationsResponse = `{
  "hitCount": 1,
  "citationList": {"citation": [
    {"id": "999", "source": "MED", "title": "Citing article", "citedByCount": 4}
  ]}
}`

const epmcDatabaseLinksResponse = `{
  "hitCount": 1,
  "dbCrossReferenceList": {"dbCrossReference": [
    {"dbName": "UNIPROT", "dbCount": 2,
     "dbCrossReferenceInfo": [{"info1": "P12345"}, {"info1": "Q67890"}]}
  ]}
}`

const epmcFullTextXML = `<?xml version="1.0" encoding="UTF-8"?>
<article article-type="research-article">
  <front>
    <journal-meta>
      <journal-title-group><journal-title>Test Journal</journal-title></journal-title-group>
    </journal-meta>
    <article-meta>
      <article-id pub-id-type="pmcid">PMC7906746</article-id>
      <article-id pub-id-type="doi">10.1234/test.2021</article-id>
      <title-group><article-title>A Europe PMC full-text article</article-title></title-group>
    </article-meta>
  </front>
  <body>
    <sec><title>Introduction</title><p>Some body text.</p></sec>
  </body>
</article>`

// newEuropePMCStubClient starts a stub Europe PMC server and returns a client
// pointed at it. The server and the client are cleaned up when the test ends.
func newEuropePMCStubClient(t *testing.T) *Client {
	t.Helper()

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		path := r.URL.Path
		switch {
		case strings.HasSuffix(path, "/search"):
			writeStub(w, "application/json", epmcSearchResponse)
		case strings.HasSuffix(path, "/references"):
			writeStub(w, "application/json", epmcReferencesResponse)
		case strings.HasSuffix(path, "/citations"):
			writeStub(w, "application/json", epmcCitationsResponse)
		case strings.HasSuffix(path, "/databaseLinks"):
			writeStub(w, "application/json", epmcDatabaseLinksResponse)
		case strings.HasSuffix(path, "/fullTextXML"):
			writeStub(w, "application/xml", epmcFullTextXML)
		default:
			http.NotFound(w, r)
		}
	}))
	t.Cleanup(server.Close)

	client, err := New(&Config{
		EuropePMCBaseURL: server.URL,
		Tool:             "pubmed-client-go-tests",
		RateLimit:        100,
		Timeout:          30 * time.Second,
	})
	if err != nil {
		t.Fatalf("New failed: %v", err)
	}
	t.Cleanup(func() { _ = client.Close() })

	return client
}

func TestEuropePMCSearch(t *testing.T) {
	client := newEuropePMCStubClient(t)

	results, err := client.EuropePMCSearch(context.Background(), "malaria vaccine", 2)
	if err != nil {
		t.Fatalf("EuropePMCSearch failed: %v", err)
	}
	if len(results) != 2 {
		t.Fatalf("expected 2 results, got %d", len(results))
	}

	first := results[0]
	if first.Source != "MED" || first.ID != "33515491" {
		t.Errorf("unexpected address: %s/%s", first.Source, first.ID)
	}
	if first.EuropePMCID != "MED/33515491" {
		t.Errorf("EuropePMCID = %q, want MED/33515491", first.EuropePMCID)
	}
	if first.PMCID != "PMC7894017" || first.PubYear != "2021" {
		t.Errorf("unexpected metadata: pmcid=%q pubYear=%q", first.PMCID, first.PubYear)
	}
	if !first.OpenAccess() {
		t.Error("expected the record to be flagged open access")
	}
	// Fields Europe PMC returns but that are not modelled must survive rather
	// than being dropped on the way through Go's decoder.
	if got, ok := first.Extra["citedByCount"]; !ok || got != float64(42) {
		t.Errorf("Extra[citedByCount] = %v (present=%v), want 42", got, ok)
	}

	if results[1].Source != "PPR" {
		t.Errorf("second result source = %q, want PPR", results[1].Source)
	}
}

func TestEuropePMCSearchPage(t *testing.T) {
	client := newEuropePMCStubClient(t)

	page, err := client.EuropePMCSearchPage(context.Background(), "malaria vaccine",
		EuropePMCSearchOptions{ResultType: EuropePMCCore, PageSize: 2})
	if err != nil {
		t.Fatalf("EuropePMCSearchPage failed: %v", err)
	}
	if page.HitCount != 2 {
		t.Errorf("HitCount = %d, want 2", page.HitCount)
	}
	if page.NextCursorMark != "AoJ456" {
		t.Errorf("NextCursorMark = %q, want AoJ456", page.NextCursorMark)
	}
	if len(page.Results) != 2 {
		t.Errorf("expected 2 results on the page, got %d", len(page.Results))
	}
}

func TestEuropePMCSearchRejectsUnknownResultType(t *testing.T) {
	client := newEuropePMCStubClient(t)

	_, err := client.EuropePMCSearchWithOptions(context.Background(), "cancer", 1,
		EuropePMCSearchOptions{ResultType: "verbose"})
	if !errors.Is(err, ErrInvalidArgument) {
		t.Fatalf("EuropePMCSearchWithOptions(result_type=verbose) = %v, want ErrInvalidArgument", err)
	}
}

func TestEuropePMCFetchFullText(t *testing.T) {
	client := newEuropePMCStubClient(t)

	article, err := client.EuropePMCFetchFullText(context.Background(), "PMC7906746", "")
	if err != nil {
		t.Fatalf("EuropePMCFetchFullText failed: %v", err)
	}
	if article.PMCID != "PMC7906746" {
		t.Errorf("PMCID = %q, want PMC7906746", article.PMCID)
	}
	if article.Title != "A Europe PMC full-text article" {
		t.Errorf("unexpected title: %q", article.Title)
	}
}

func TestEuropePMCFetchXML(t *testing.T) {
	client := newEuropePMCStubClient(t)

	xml, err := client.EuropePMCFetchXML(context.Background(), "PMC7906746", "")
	if err != nil {
		t.Fatalf("EuropePMCFetchXML failed: %v", err)
	}
	if !strings.Contains(xml, "<article") {
		t.Errorf("expected JATS XML, got %.60q", xml)
	}
}

func TestEuropePMCReferences(t *testing.T) {
	client := newEuropePMCStubClient(t)

	references, err := client.EuropePMCReferences(context.Background(), "33515491", "MED")
	if err != nil {
		t.Fatalf("EuropePMCReferences failed: %v", err)
	}
	if len(references) != 2 {
		t.Fatalf("expected 2 references, got %d", len(references))
	}
	// pubYear arrives as a JSON number and is normalized to a string upstream.
	if references[0].PubYear != "2010" || references[0].PMID != "12345" {
		t.Errorf("unexpected first reference: %+v", references[0])
	}
	if references[1].ID != "" {
		t.Errorf("an unmatched reference should carry no id, got %q", references[1].ID)
	}
}

func TestEuropePMCCitations(t *testing.T) {
	client := newEuropePMCStubClient(t)

	citations, err := client.EuropePMCCitations(context.Background(), "PMC7906746", "")
	if err != nil {
		t.Fatalf("EuropePMCCitations failed: %v", err)
	}
	if len(citations) != 1 {
		t.Fatalf("expected 1 citation, got %d", len(citations))
	}
	if citations[0].CitedByCount != "4" {
		t.Errorf("CitedByCount = %q, want 4", citations[0].CitedByCount)
	}
}

func TestEuropePMCDatabaseLinks(t *testing.T) {
	client := newEuropePMCStubClient(t)

	links, err := client.EuropePMCDatabaseLinks(context.Background(), "33515491", "MED")
	if err != nil {
		t.Fatalf("EuropePMCDatabaseLinks failed: %v", err)
	}
	if len(links) != 1 {
		t.Fatalf("expected 1 database link group, got %d", len(links))
	}
	if links[0].DBName != "UNIPROT" || links[0].DBCount != 2 {
		t.Errorf("unexpected group: %+v", links[0])
	}
	if len(links[0].Info) != 2 || links[0].Info[0].Info1 != "P12345" {
		t.Errorf("unexpected cross-references: %+v", links[0].Info)
	}
}

// The record address is validated before any request is issued, so these need
// no server at all.
func TestEuropePMCRejectsInvalidRecordIDs(t *testing.T) {
	client := newEuropePMCStubClient(t)

	cases := []struct {
		name   string
		id     string
		source string
	}{
		{"empty id", "   ", ""},
		{"malformed qualified id", "MED/", ""},
		{"non-numeric PMC id", "not-a-pmcid", "PMC"},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			_, err := client.EuropePMCReferences(context.Background(), tc.id, tc.source)
			if !errors.Is(err, ErrInvalidArgument) {
				t.Fatalf("EuropePMCReferences(%q, %q) = %v, want ErrInvalidArgument", tc.id, tc.source, err)
			}
		})
	}
}

func TestEuropePMCSupplementaryRequiresOutputPath(t *testing.T) {
	client := newEuropePMCStubClient(t)

	_, err := client.EuropePMCDownloadSupplementaryFiles(context.Background(), "PMC7906746", "", "")
	if !errors.Is(err, ErrInvalidArgument) {
		t.Fatalf("EuropePMCDownloadSupplementaryFiles(outputPath=\"\") = %v, want ErrInvalidArgument", err)
	}
}
