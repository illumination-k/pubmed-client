package pubmedclient

import (
	"context"
	"encoding/json"
	"errors"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

func TestFetchFullTextAgainstStub(t *testing.T) {
	client := newStubClient(t, defaultStub())

	article, err := client.FetchFullText(context.Background(), "PMC7906746")
	if err != nil {
		t.Fatalf("FetchFullText failed: %v", err)
	}

	if article.PMCID != "PMC7906746" {
		t.Errorf("PMCID = %q", article.PMCID)
	}
	if article.Title != "A full-text article for the Go tests" {
		t.Errorf("Title = %q", article.Title)
	}
	if article.DOI != "10.1234/test.2021" {
		t.Errorf("DOI = %q", article.DOI)
	}
	if article.Journal.Title != "Test Journal" {
		t.Errorf("Journal.Title = %q", article.Journal.Title)
	}
	if len(article.Keywords) != 2 {
		t.Errorf("Keywords = %v, want two entries", article.Keywords)
	}
	// The abstract is surfaced as the first section, ahead of the body's own.
	if len(article.Sections) != 3 {
		t.Fatalf("got %d sections, want 3 (abstract plus two body sections)", len(article.Sections))
	}
	if article.Sections[0].SectionType != "abstract" {
		t.Errorf("Sections[0].SectionType = %q, want %q", article.Sections[0].SectionType, "abstract")
	}
	if article.Sections[1].Title != "Introduction" {
		t.Errorf("Sections[1].Title = %q", article.Sections[1].Title)
	}
	if len(article.References) != 1 {
		t.Errorf("got %d references, want 1", len(article.References))
	}
	// Figures are flattened out of the section tree, so the top-level slice and
	// the count must agree.
	if article.FigureCount != 1 || len(article.Figures) != 1 {
		t.Fatalf("FigureCount = %d, len(Figures) = %d, want 1 and 1",
			article.FigureCount, len(article.Figures))
	}
	if article.Figures[0].ID != "fig1" {
		t.Errorf("Figures[0].ID = %q", article.Figures[0].ID)
	}
}

func TestFetchXMLReturnsTheRawDocument(t *testing.T) {
	client := newStubClient(t, defaultStub())

	xml, err := client.FetchXML(context.Background(), "PMC7906746")
	if err != nil {
		t.Fatalf("FetchXML failed: %v", err)
	}
	if !strings.Contains(xml, "<pmc-articleset>") {
		t.Errorf("FetchXML did not return JATS XML: %.80q", xml)
	}
}

func TestFetchMarkdownAgainstStub(t *testing.T) {
	client := newStubClient(t, defaultStub())

	markdown, err := client.FetchMarkdown(context.Background(), "PMC7906746")
	if err != nil {
		t.Fatalf("FetchMarkdown failed: %v", err)
	}

	if !strings.Contains(markdown, "A full-text article for the Go tests") {
		t.Errorf("markdown is missing the title: %.200q", markdown)
	}
	if !strings.Contains(markdown, "Introduction") {
		t.Errorf("markdown is missing the first section: %.200q", markdown)
	}
	// The default rendering uses bold metadata rather than YAML frontmatter.
	if strings.HasPrefix(markdown, "---") {
		t.Error("default markdown starts with YAML frontmatter")
	}
}

func TestFetchMarkdownWithOptionsChangesTheRendering(t *testing.T) {
	client := newStubClient(t, defaultStub())

	markdown, err := client.FetchMarkdownWithOptions(context.Background(), "PMC7906746",
		MarkdownOptions{
			YAMLFrontmatter: Bool(true),
			IncludeTOC:      Bool(true),
		})
	if err != nil {
		t.Fatalf("FetchMarkdownWithOptions failed: %v", err)
	}

	if !strings.HasPrefix(markdown, "---") {
		t.Errorf("YAMLFrontmatter did not take effect: %.200q", markdown)
	}
}

func TestFetchMarkdownWithoutMetadataOmitsIt(t *testing.T) {
	client := newStubClient(t, defaultStub())
	ctx := context.Background()

	withMetadata, err := client.FetchMarkdown(ctx, "PMC7906746")
	if err != nil {
		t.Fatalf("FetchMarkdown failed: %v", err)
	}

	// IncludeMetadata defaults to true, so a plain bool could not express this;
	// the pointer is what distinguishes "off" from "unset".
	without, err := client.FetchMarkdownWithOptions(ctx, "PMC7906746",
		MarkdownOptions{IncludeMetadata: Bool(false)})
	if err != nil {
		t.Fatalf("FetchMarkdownWithOptions failed: %v", err)
	}

	if len(without) >= len(withMetadata) {
		t.Errorf("IncludeMetadata=false did not shorten the output (%d vs %d bytes)",
			len(without), len(withMetadata))
	}
}

func TestFetchMarkdownRejectsUnknownStyles(t *testing.T) {
	client := newStubClient(t, defaultStub())
	ctx := context.Background()

	_, err := client.FetchMarkdownWithOptions(ctx, "PMC7906746",
		MarkdownOptions{HeadingStyle: HeadingStyle("underline")})
	if !errors.Is(err, ErrInvalidArgument) {
		t.Errorf("unknown heading style = %v, want ErrInvalidArgument", err)
	}

	_, err = client.FetchMarkdownWithOptions(ctx, "PMC7906746",
		MarkdownOptions{ReferenceStyle: ReferenceStyle("harvard")})
	if !errors.Is(err, ErrInvalidArgument) {
		t.Errorf("unknown reference style = %v, want ErrInvalidArgument", err)
	}

	_, err = client.FetchMarkdownWithOptions(ctx, "PMC7906746",
		MarkdownOptions{MaxHeadingLevel: 9})
	if !errors.Is(err, ErrInvalidArgument) {
		t.Errorf("out-of-range heading level = %v, want ErrInvalidArgument", err)
	}
}

// The zero MarkdownOptions must encode to an empty object, so it means "use the
// defaults" rather than "turn everything off".
func TestZeroMarkdownOptionsEncodeToAnEmptyObject(t *testing.T) {
	encoded, err := json.Marshal(MarkdownOptions{})
	if err != nil {
		t.Fatalf("marshal failed: %v", err)
	}
	if string(encoded) != "{}" {
		t.Errorf("zero MarkdownOptions encoded as %s, want {}", encoded)
	}
}

func TestCheckPMCAvailabilityAgainstStub(t *testing.T) {
	client := newStubClient(t, defaultStub())

	pmcid, available, err := client.CheckPMCAvailability(context.Background(), "31978945")
	if err != nil {
		t.Fatalf("CheckPMCAvailability failed: %v", err)
	}
	if !available {
		t.Fatal("available = false, want true (the stub advertises PMC links)")
	}
	if pmcid != "PMC7092803" {
		t.Errorf("pmcid = %q, want %q", pmcid, "PMC7092803")
	}
}

func TestCheckPMCAvailabilityWithoutLinks(t *testing.T) {
	// A PMID with no PMC full text gets an ELink response with no linksetdbs,
	// which must read as "unavailable" rather than as an error.
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		writeStub(w, "application/json", `{"linksets":[{"dbfrom":"pubmed","ids":["1"]}]}`)
	}))
	defer server.Close()

	client, err := New(&Config{BaseURL: server.URL, RateLimit: 100})
	if err != nil {
		t.Fatalf("New failed: %v", err)
	}
	defer client.Close()

	pmcid, available, err := client.CheckPMCAvailability(context.Background(), "31978945")
	if err != nil {
		t.Fatalf("CheckPMCAvailability failed: %v", err)
	}
	if available || pmcid != "" {
		t.Errorf("got (%q, %v), want (\"\", false)", pmcid, available)
	}
}

// An article outside the Open Access subset must be distinguishable from a
// transport failure, since it is the expected outcome for most articles.
func TestUnavailableFullTextMatchesErrPMCNotAvailable(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		writeStub(w, "application/xml",
			`<?xml version="1.0"?><pmc-articleset><ERROR>Article not available</ERROR></pmc-articleset>`)
	}))
	defer server.Close()

	client, err := New(&Config{BaseURL: server.URL, RateLimit: 100})
	if err != nil {
		t.Fatalf("New failed: %v", err)
	}
	defer client.Close()

	_, err = client.FetchFullText(context.Background(), "PMC1")
	if !errors.Is(err, ErrPMCNotAvailable) {
		t.Fatalf("FetchFullText = %v, want ErrPMCNotAvailable", err)
	}
}

func TestDownloadsRejectAnEmptyOutputDir(t *testing.T) {
	client := newStubClient(t, defaultStub())
	ctx := context.Background()

	if _, err := client.DownloadFiles(ctx, "PMC7906746", ""); !errors.Is(err, ErrInvalidArgument) {
		t.Errorf("DownloadFiles with no directory = %v, want ErrInvalidArgument", err)
	}
	if _, err := client.ExtractFigures(ctx, "PMC7906746", ""); !errors.Is(err, ErrInvalidArgument) {
		t.Errorf("ExtractFigures with no directory = %v, want ErrInvalidArgument", err)
	}
}

func TestClearPMCCacheIsANoOpWithoutCaching(t *testing.T) {
	client := newStubClient(t, defaultStub())

	if err := client.ClearPMCCache(context.Background()); err != nil {
		t.Fatalf("ClearPMCCache failed: %v", err)
	}
}

// The extracted-figure decoding has two hand-written pieces — the [w, h] pair
// and the optional file size — so it is checked directly rather than through a
// download.
func TestExtractedFigureDecoding(t *testing.T) {
	const payload = `[
	  {"figure": {"id": "fig1", "label": "Figure 1", "caption": "A caption"},
	   "extracted_file_path": "/tmp/fig1.jpg", "file_size": 2048, "dimensions": [800, 600]},
	  {"figure": {"id": "fig2"}, "extracted_file_path": "/tmp/fig2.jpg",
	   "file_size": null, "dimensions": null}
	]`

	var figures []ExtractedFigure
	if err := json.Unmarshal([]byte(payload), &figures); err != nil {
		t.Fatalf("decode failed: %v", err)
	}
	if len(figures) != 2 {
		t.Fatalf("got %d figures, want 2", len(figures))
	}

	first := figures[0]
	if first.Figure.ID != "fig1" || first.Figure.Caption != "A caption" {
		t.Errorf("Figure = %+v", first.Figure)
	}
	if first.Path != "/tmp/fig1.jpg" {
		t.Errorf("Path = %q", first.Path)
	}
	if first.FileSize == nil || *first.FileSize != 2048 {
		t.Errorf("FileSize = %v", first.FileSize)
	}
	if first.Dimensions == nil {
		t.Fatal("Dimensions is nil")
	}
	if first.Dimensions.Width != 800 || first.Dimensions.Height != 600 {
		t.Errorf("Dimensions = %+v, want 800x600", *first.Dimensions)
	}

	// Unknown size and dimensions must decode as nil, not as zeroes.
	if figures[1].FileSize != nil || figures[1].Dimensions != nil {
		t.Errorf("second figure = %+v, want nil size and dimensions", figures[1])
	}
}

func TestOASubsetInfoDecoding(t *testing.T) {
	const payload = `{"pmcid": "PMC7906746", "is_oa_subset": true, "license": "CC BY",
	                  "retracted": false, "download_link": "https://example.test/x.tgz",
	                  "download_format": "tgz"}`

	var info OASubsetInfo
	if err := json.Unmarshal([]byte(payload), &info); err != nil {
		t.Fatalf("decode failed: %v", err)
	}
	if !info.IsOASubset || info.License != "CC BY" || info.DownloadFormat != "tgz" {
		t.Errorf("OASubsetInfo = %+v", info)
	}
}
