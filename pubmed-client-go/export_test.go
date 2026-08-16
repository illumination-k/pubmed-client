package pubmedclient

import (
	"encoding/json"
	"errors"
	"strings"
	"testing"
)

// Export is a pure function on the Rust side, so these tests need no client.

func sampleArticle() Article {
	return Article{
		PMID:        "31978945",
		Title:       "A test article about CRISPR",
		AuthorCount: 2,
		Authors: []Author{
			{Surname: "Doe", GivenNames: "Jane", FullName: "Jane Doe"},
			{Surname: "Roe", GivenNames: "Richard", FullName: "Richard Roe"},
		},
		Journal: "Nature",
		PubDate: "2020",
		DOI:     "10.1038/s41586-020-0000-0",
		Volume:  "578",
		Issue:   "7793",
		Pages:   "82-93",
	}
}

func TestExportBibTeX(t *testing.T) {
	got, err := ExportArticles([]Article{sampleArticle()}, FormatBibTeX)
	if err != nil {
		t.Fatalf("ExportArticles failed: %v", err)
	}

	for _, want := range []string{"@article{", "title = {", "Nature", "10.1038"} {
		if !strings.Contains(got, want) {
			t.Errorf("BibTeX output is missing %q:\n%s", want, got)
		}
	}
}

func TestExportRIS(t *testing.T) {
	got, err := ExportArticles([]Article{sampleArticle()}, FormatRIS)
	if err != nil {
		t.Fatalf("ExportArticles failed: %v", err)
	}
	if !strings.Contains(got, "TY  -") || !strings.Contains(got, "ER  -") {
		t.Errorf("RIS output is missing its record markers:\n%s", got)
	}
}

func TestExportNBIB(t *testing.T) {
	got, err := ExportArticles([]Article{sampleArticle()}, FormatNBIB)
	if err != nil {
		t.Fatalf("ExportArticles failed: %v", err)
	}
	if !strings.Contains(got, "PMID- 31978945") {
		t.Errorf("NBIB output is missing the PMID line:\n%s", got)
	}
}

func TestExportCSLJSON(t *testing.T) {
	got, err := ExportArticles([]Article{sampleArticle()}, FormatCSLJSON)
	if err != nil {
		t.Fatalf("ExportArticles failed: %v", err)
	}

	var parsed []map[string]any
	if err := json.Unmarshal([]byte(got), &parsed); err != nil {
		t.Fatalf("CSL-JSON is not a JSON array: %v\n%s", err, got)
	}
	if len(parsed) != 1 {
		t.Fatalf("got %d CSL-JSON entries, want 1", len(parsed))
	}
	if parsed[0]["title"] != "A test article about CRISPR" {
		t.Errorf("CSL-JSON title = %v", parsed[0]["title"])
	}
}

func TestExportSeveralArticles(t *testing.T) {
	second := sampleArticle()
	second.PMID = "33515491"
	second.Title = "Another test article"

	got, err := ExportArticles([]Article{sampleArticle(), second}, FormatBibTeX)
	if err != nil {
		t.Fatalf("ExportArticles failed: %v", err)
	}
	if count := strings.Count(got, "@article{"); count != 2 {
		t.Errorf("got %d BibTeX entries, want 2:\n%s", count, got)
	}
}

// A hand-built article with only the required fields must export rather than
// fail: Go omits unset optional fields, and the Rust side fills in defaults.
func TestExportAcceptsAMinimalArticle(t *testing.T) {
	minimal := Article{PMID: "1", Title: "T", Journal: "J", PubDate: "2020"}

	got, err := ExportArticles([]Article{minimal}, FormatBibTeX)
	if err != nil {
		t.Fatalf("ExportArticles failed: %v", err)
	}
	if !strings.Contains(got, "@article{") {
		t.Errorf("minimal article did not export:\n%s", got)
	}
}

func TestExportNothingYieldsAnEmptyDocument(t *testing.T) {
	got, err := ExportArticles(nil, FormatBibTeX)
	if err != nil {
		t.Fatalf("ExportArticles(nil) failed: %v", err)
	}
	if got != "" {
		t.Errorf("ExportArticles(nil) = %q, want an empty string", got)
	}
}

func TestExportRejectsAnUnknownFormat(t *testing.T) {
	_, err := ExportArticles([]Article{sampleArticle()}, ExportFormat("markdown"))
	if !errors.Is(err, ErrInvalidArgument) {
		t.Fatalf("ExportArticles = %v, want ErrInvalidArgument", err)
	}

	var typed *Error
	if errors.As(err, &typed) && !strings.Contains(typed.Message, "bibtex") {
		t.Errorf("message %q does not list the supported formats", typed.Message)
	}
}

func TestArticleExportHelpers(t *testing.T) {
	article := sampleArticle()

	helpers := map[string]func() (string, error){
		"ToBibTeX":  article.ToBibTeX,
		"ToRIS":     article.ToRIS,
		"ToCSLJSON": article.ToCSLJSON,
		"ToNBIB":    article.ToNBIB,
	}

	for name, helper := range helpers {
		got, err := helper()
		if err != nil {
			t.Errorf("%s failed: %v", name, err)
			continue
		}
		if got == "" {
			t.Errorf("%s returned an empty document", name)
		}
	}
}

func TestArticleExportRejectsNil(t *testing.T) {
	var article *Article
	if _, err := article.ToBibTeX(); !errors.Is(err, ErrInvalidArgument) {
		t.Errorf("ToBibTeX on a nil article = %v, want ErrInvalidArgument", err)
	}
}

// Articles fetched from PubMed must round-trip: whatever the decoder produced
// has to be re-encodable into something the Rust exporter accepts.
func TestFetchedArticlesExport(t *testing.T) {
	client := newStubClient(t, defaultStub())

	articles, err := client.SearchAndFetch(t.Context(), "CRISPR", 2)
	if err != nil {
		t.Fatalf("SearchAndFetch failed: %v", err)
	}

	got, err := ExportArticles(articles, FormatBibTeX)
	if err != nil {
		t.Fatalf("ExportArticles failed: %v", err)
	}
	if !strings.Contains(got, "31978945") && !strings.Contains(got, "CRISPR") {
		t.Errorf("exported citation does not mention the article:\n%s", got)
	}
}
