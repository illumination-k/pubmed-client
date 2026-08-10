package pubmedclient

import (
	"encoding/json"
	"errors"
	"net/http"
	"net/http/httptest"
	"strings"
	"sync"
	"testing"
	"time"
)

func TestVersionIsPopulated(t *testing.T) {
	if Version() == "" {
		t.Fatal("Version() returned an empty string")
	}
}

func TestNewWithNilConfig(t *testing.T) {
	client, err := New(nil)
	if err != nil {
		t.Fatalf("New(nil) failed: %v", err)
	}
	if err := client.Close(); err != nil {
		t.Fatalf("Close() failed: %v", err)
	}
}

func TestCloseIsIdempotent(t *testing.T) {
	client, err := New(nil)
	if err != nil {
		t.Fatalf("New failed: %v", err)
	}
	for i := 0; i < 3; i++ {
		if err := client.Close(); err != nil {
			t.Fatalf("Close() call %d failed: %v", i, err)
		}
	}
}

func TestCallsAfterCloseReturnErrClosed(t *testing.T) {
	client, err := New(nil)
	if err != nil {
		t.Fatalf("New failed: %v", err)
	}
	if err := client.Close(); err != nil {
		t.Fatalf("Close failed: %v", err)
	}

	if _, err := client.SearchArticles("cancer", 1); !errors.Is(err, ErrClosed) {
		t.Errorf("SearchArticles after Close = %v, want ErrClosed", err)
	}
	if _, err := client.FetchArticle("31978945"); !errors.Is(err, ErrClosed) {
		t.Errorf("FetchArticle after Close = %v, want ErrClosed", err)
	}
	if _, err := client.FetchArticles([]string{"31978945"}); !errors.Is(err, ErrClosed) {
		t.Errorf("FetchArticles after Close = %v, want ErrClosed", err)
	}
	if _, err := client.FetchFullText("PMC7906746"); !errors.Is(err, ErrClosed) {
		t.Errorf("FetchFullText after Close = %v, want ErrClosed", err)
	}
	if _, err := client.FetchMarkdown("PMC7906746"); !errors.Is(err, ErrClosed) {
		t.Errorf("FetchMarkdown after Close = %v, want ErrClosed", err)
	}
	if _, _, err := client.CheckPMCAvailability("31978945"); !errors.Is(err, ErrClosed) {
		t.Errorf("CheckPMCAvailability after Close = %v, want ErrClosed", err)
	}
}

func TestNewAcceptsFullConfig(t *testing.T) {
	client, err := New(&Config{
		APIKey:    "test-key",
		Email:     "test@example.com",
		Tool:      "pubmed-client-go-tests",
		RateLimit: 5,
		Timeout:   15 * time.Second,
		UserAgent: "pubmed-client-go/test",
		BaseURL:   "https://example.invalid",
		Cache:     true,
	})
	if err != nil {
		t.Fatalf("New with full config failed: %v", err)
	}
	defer client.Close()
}

func TestRejectsNonPositiveLimit(t *testing.T) {
	client, err := New(nil)
	if err != nil {
		t.Fatalf("New failed: %v", err)
	}
	defer client.Close()

	for _, limit := range []int{0, -1} {
		if _, err := client.SearchArticles("cancer", limit); err == nil {
			t.Errorf("SearchArticles(limit=%d) succeeded, want error", limit)
		}
		if _, err := client.SearchAndFetch("cancer", limit); err == nil {
			t.Errorf("SearchAndFetch(limit=%d) succeeded, want error", limit)
		}
	}
}

func TestFetchArticlesWithNoPMIDsSkipsTheCall(t *testing.T) {
	client, err := New(&Config{BaseURL: "https://example.invalid"})
	if err != nil {
		t.Fatalf("New failed: %v", err)
	}
	defer client.Close()

	// example.invalid never resolves, so a non-error here proves no request was
	// made.
	articles, err := client.FetchArticles(nil)
	if err != nil {
		t.Fatalf("FetchArticles(nil) failed: %v", err)
	}
	if len(articles) != 0 {
		t.Errorf("FetchArticles(nil) returned %d articles, want 0", len(articles))
	}
}

func TestConfigMarshalOmitsUnsetFields(t *testing.T) {
	var nilConfig *Config
	if encoded, err := nilConfig.marshal(); err != nil || encoded != "" {
		t.Fatalf("nil Config marshal = (%q, %v), want (\"\", nil)", encoded, err)
	}

	encoded, err := (&Config{}).marshal()
	if err != nil {
		t.Fatalf("empty Config marshal failed: %v", err)
	}
	var payload map[string]any
	if err := json.Unmarshal([]byte(encoded), &payload); err != nil {
		t.Fatalf("marshal produced invalid JSON %q: %v", encoded, err)
	}
	// Only `cache` is unconditional; every other key must be absent so the Rust
	// side falls back to its own defaults.
	if len(payload) != 1 {
		t.Errorf("empty Config encoded %d keys (%v), want only \"cache\"", len(payload), payload)
	}
	if payload["cache"] != false {
		t.Errorf("cache = %v, want false", payload["cache"])
	}
}

func TestConfigMarshalRoundsTimeoutUp(t *testing.T) {
	tests := []struct {
		timeout time.Duration
		want    float64
	}{
		{500 * time.Millisecond, 1},
		{time.Second, 1},
		{1500 * time.Millisecond, 2},
		{30 * time.Second, 30},
	}

	for _, test := range tests {
		encoded, err := (&Config{Timeout: test.timeout}).marshal()
		if err != nil {
			t.Fatalf("marshal failed: %v", err)
		}
		var payload map[string]any
		if err := json.Unmarshal([]byte(encoded), &payload); err != nil {
			t.Fatalf("invalid JSON: %v", err)
		}
		if got := payload["timeout_seconds"]; got != test.want {
			t.Errorf("Timeout %v encoded as %v, want %v", test.timeout, got, test.want)
		}
	}
}

func TestErrorMessageIncludesOp(t *testing.T) {
	err := &Error{Op: "FetchArticle", Message: "boom"}
	if got, want := err.Error(), "pubmedclient.FetchArticle: boom"; got != want {
		t.Errorf("Error() = %q, want %q", got, want)
	}
}

// --- Offline end-to-end coverage --------------------------------------------
//
// BaseURL points the underlying client at a local server, so these exercise the
// whole chain (Go -> cgo -> Rust -> HTTP -> XML parsing -> JSON -> Go structs)
// without touching NCBI.

const esearchResponse = `{"esearchresult":{"count":"2","retmax":"2","retstart":"0","idlist":["31978945","33515491"]}}`

const efetchResponse = `<?xml version="1.0" encoding="UTF-8"?>
<PubmedArticleSet>
  <PubmedArticle>
    <MedlineCitation>
      <PMID Version="1">31978945</PMID>
      <Article PubModel="Print">
        <Journal>
          <ISSN IssnType="Electronic">1476-4687</ISSN>
          <JournalIssue CitedMedium="Internet">
            <Volume>578</Volume>
            <Issue>7793</Issue>
            <PubDate><Year>2020</Year><Month>Feb</Month></PubDate>
          </JournalIssue>
          <Title>Nature</Title>
          <ISOAbbreviation>Nature</ISOAbbreviation>
        </Journal>
        <ArticleTitle>A test article about CRISPR.</ArticleTitle>
        <Pagination><MedlinePgn>82-93</MedlinePgn></Pagination>
        <Abstract>
          <AbstractText>An abstract used by the Go binding tests.</AbstractText>
        </Abstract>
        <AuthorList CompleteYN="Y">
          <Author ValidYN="Y">
            <LastName>Doe</LastName>
            <ForeName>Jane</ForeName>
            <Initials>J</Initials>
          </Author>
          <Author ValidYN="Y">
            <LastName>Roe</LastName>
            <ForeName>Richard</ForeName>
            <Initials>R</Initials>
          </Author>
        </AuthorList>
        <Language>eng</Language>
        <PublicationTypeList>
          <PublicationType UI="D016428">Journal Article</PublicationType>
        </PublicationTypeList>
      </Article>
    </MedlineCitation>
    <PubmedData>
      <ArticleIdList>
        <ArticleId IdType="pubmed">31978945</ArticleId>
        <ArticleId IdType="doi">10.1038/s41586-020-0000-0</ArticleId>
      </ArticleIdList>
    </PubmedData>
  </PubmedArticle>
</PubmedArticleSet>`

// newStubClient starts a stub E-utilities server and returns a client pointed at
// it. The server and the client are cleaned up when the test ends.
func newStubClient(t *testing.T) *Client {
	t.Helper()

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch {
		case strings.HasSuffix(r.URL.Path, "/esearch.fcgi"):
			w.Header().Set("Content-Type", "application/json")
			_, _ = w.Write([]byte(esearchResponse))
		case strings.HasSuffix(r.URL.Path, "/efetch.fcgi"):
			w.Header().Set("Content-Type", "application/xml")
			_, _ = w.Write([]byte(efetchResponse))
		default:
			http.NotFound(w, r)
		}
	}))
	t.Cleanup(server.Close)

	client, err := New(&Config{
		BaseURL: server.URL,
		Tool:    "pubmed-client-go-tests",
		// Keep the token bucket from slowing the suite down.
		RateLimit: 100,
		Timeout:   30 * time.Second,
	})
	if err != nil {
		t.Fatalf("New failed: %v", err)
	}
	t.Cleanup(func() { _ = client.Close() })

	return client
}

func TestSearchArticlesAgainstStub(t *testing.T) {
	client := newStubClient(t)

	pmids, err := client.SearchArticles("CRISPR", 10)
	if err != nil {
		t.Fatalf("SearchArticles failed: %v", err)
	}

	want := []string{"31978945", "33515491"}
	if len(pmids) != len(want) {
		t.Fatalf("got %d pmids (%v), want %d", len(pmids), pmids, len(want))
	}
	for i := range want {
		if pmids[i] != want[i] {
			t.Errorf("pmids[%d] = %q, want %q", i, pmids[i], want[i])
		}
	}
}

func TestFetchArticleAgainstStub(t *testing.T) {
	client := newStubClient(t)

	article, err := client.FetchArticle("31978945")
	if err != nil {
		t.Fatalf("FetchArticle failed: %v", err)
	}

	if article.PMID != "31978945" {
		t.Errorf("PMID = %q, want %q", article.PMID, "31978945")
	}
	if article.Title != "A test article about CRISPR." {
		t.Errorf("Title = %q", article.Title)
	}
	if article.Journal != "Nature" {
		t.Errorf("Journal = %q, want %q", article.Journal, "Nature")
	}
	if article.DOI != "10.1038/s41586-020-0000-0" {
		t.Errorf("DOI = %q", article.DOI)
	}
	if article.AbstractText != "An abstract used by the Go binding tests." {
		t.Errorf("AbstractText = %q", article.AbstractText)
	}
	if article.AuthorCount != 2 {
		t.Errorf("AuthorCount = %d, want 2", article.AuthorCount)
	}
	if len(article.Authors) != 2 {
		t.Fatalf("got %d authors, want 2", len(article.Authors))
	}
	if article.Authors[0].FullName == "" {
		t.Error("first author FullName is empty")
	}
	if article.Authors[0].Surname != "Doe" {
		t.Errorf("Authors[0].Surname = %q, want %q", article.Authors[0].Surname, "Doe")
	}
	if article.Volume != "578" {
		t.Errorf("Volume = %q, want %q", article.Volume, "578")
	}
	if article.Language != "eng" {
		t.Errorf("Language = %q, want %q", article.Language, "eng")
	}
	if len(article.ArticleTypes) == 0 {
		t.Error("ArticleTypes is empty")
	}
}

func TestSearchAndFetchAgainstStub(t *testing.T) {
	client := newStubClient(t)

	articles, err := client.SearchAndFetch("CRISPR", 2)
	if err != nil {
		t.Fatalf("SearchAndFetch failed: %v", err)
	}
	if len(articles) != 1 {
		t.Fatalf("got %d articles, want 1 (the stub returns one record)", len(articles))
	}
	if articles[0].PMID != "31978945" {
		t.Errorf("PMID = %q", articles[0].PMID)
	}
}

func TestErrorsFromTheRustSideSurfaceAsError(t *testing.T) {
	// 400 rather than 500: the underlying client retries 5xx with backoff, which
	// would add seconds to the suite for no extra coverage.
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		http.Error(w, "bad request", http.StatusBadRequest)
	}))
	defer server.Close()

	client, err := New(&Config{BaseURL: server.URL, RateLimit: 100})
	if err != nil {
		t.Fatalf("New failed: %v", err)
	}
	defer client.Close()

	_, err = client.SearchArticles("CRISPR", 1)
	if err == nil {
		t.Fatal("SearchArticles against a failing server succeeded, want error")
	}

	var ffiErr *Error
	if !errors.As(err, &ffiErr) {
		t.Fatalf("error %v is not a *Error", err)
	}
	if ffiErr.Op != "SearchArticles" {
		t.Errorf("Op = %q, want %q", ffiErr.Op, "SearchArticles")
	}
	if ffiErr.Message == "" {
		t.Error("Message is empty")
	}
}

// Concurrent use must be safe: the Rust client is shared behind an Arc and the
// Go wrapper only guards against close-during-call.
func TestConcurrentCalls(t *testing.T) {
	client := newStubClient(t)

	const goroutines = 8
	var wg sync.WaitGroup
	errs := make(chan error, goroutines)

	for i := 0; i < goroutines; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			if _, err := client.SearchArticles("CRISPR", 5); err != nil {
				errs <- err
			}
		}()
	}

	wg.Wait()
	close(errs)
	for err := range errs {
		t.Errorf("concurrent SearchArticles failed: %v", err)
	}
}
