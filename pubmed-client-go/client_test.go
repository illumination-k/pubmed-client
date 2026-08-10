package pubmedclient

import (
	"context"
	"encoding/json"
	"errors"
	"net/http"
	"net/http/httptest"
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

	ctx := context.Background()

	// One per call family: the shared plumbing in Client.call is what refuses a
	// closed handle, so this checks the wiring rather than each method.
	calls := map[string]func() error{
		"SearchArticles": func() error {
			_, err := client.SearchArticles(ctx, "cancer", 1)
			return err
		},
		"FetchArticle": func() error {
			_, err := client.FetchArticle(ctx, "31978945")
			return err
		},
		"FetchArticles": func() error {
			_, err := client.FetchArticles(ctx, []string{"31978945"})
			return err
		},
		"FetchSummaries": func() error {
			_, err := client.FetchSummaries(ctx, []string{"31978945"})
			return err
		},
		"GetRelatedArticles": func() error {
			_, err := client.GetRelatedArticles(ctx, []uint32{31978945})
			return err
		},
		"GetDatabaseList": func() error {
			_, err := client.GetDatabaseList(ctx)
			return err
		},
		"SpellCheck": func() error {
			_, err := client.SpellCheck(ctx, "asthmaa")
			return err
		},
		"GlobalQuery": func() error {
			_, err := client.GlobalQuery(ctx, "asthma")
			return err
		},
		"MatchCitations": func() error {
			_, err := client.MatchCitations(ctx, []CitationQuery{{Journal: "nature"}})
			return err
		},
		"FetchFullText": func() error {
			_, err := client.FetchFullText(ctx, "PMC7906746")
			return err
		},
		"FetchXML": func() error {
			_, err := client.FetchXML(ctx, "PMC7906746")
			return err
		},
		"FetchMarkdown": func() error {
			_, err := client.FetchMarkdown(ctx, "PMC7906746")
			return err
		},
		"CheckPMCAvailability": func() error {
			_, _, err := client.CheckPMCAvailability(ctx, "31978945")
			return err
		},
		"IsOASubset": func() error {
			_, err := client.IsOASubset(ctx, "PMC7906746")
			return err
		},
		"DownloadFiles": func() error {
			_, err := client.DownloadFiles(ctx, "PMC7906746", t.TempDir())
			return err
		},
		"ExtractFigures": func() error {
			_, err := client.ExtractFigures(ctx, "PMC7906746", t.TempDir())
			return err
		},
		"ClearPMCCache": func() error { return client.ClearPMCCache(ctx) },
	}

	for name, call := range calls {
		if err := call(); !errors.Is(err, ErrClosed) {
			t.Errorf("%s after Close = %v, want ErrClosed", name, err)
		}
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

	ctx := context.Background()
	for _, limit := range []int{0, -1} {
		if _, err := client.SearchArticles(ctx, "cancer", limit); !errors.Is(err, ErrInvalidArgument) {
			t.Errorf("SearchArticles(limit=%d) = %v, want ErrInvalidArgument", limit, err)
		}
		if _, err := client.SearchAndFetch(ctx, "cancer", limit); !errors.Is(err, ErrInvalidArgument) {
			t.Errorf("SearchAndFetch(limit=%d) = %v, want ErrInvalidArgument", limit, err)
		}
		if _, err := client.SearchWithFullText(ctx, "cancer", limit); !errors.Is(err, ErrInvalidArgument) {
			t.Errorf("SearchWithFullText(limit=%d) = %v, want ErrInvalidArgument", limit, err)
		}
	}
}

// Calls that can be answered locally must not reach the network. BaseURL points
// at a host that never resolves, so a non-error proves no request was made.
func TestEmptyInputsSkipTheCall(t *testing.T) {
	client, err := New(&Config{BaseURL: "https://example.invalid"})
	if err != nil {
		t.Fatalf("New failed: %v", err)
	}
	defer client.Close()

	ctx := context.Background()

	articles, err := client.FetchArticles(ctx, nil)
	if err != nil {
		t.Fatalf("FetchArticles(nil) failed: %v", err)
	}
	if len(articles) != 0 {
		t.Errorf("FetchArticles(nil) returned %d articles, want 0", len(articles))
	}

	all, err := client.FetchAllByPMIDs(ctx, nil)
	if err != nil {
		t.Fatalf("FetchAllByPMIDs(nil) failed: %v", err)
	}
	if len(all) != 0 {
		t.Errorf("FetchAllByPMIDs(nil) returned %d articles, want 0", len(all))
	}

	summaries, err := client.FetchSummaries(ctx, nil)
	if err != nil {
		t.Fatalf("FetchSummaries(nil) failed: %v", err)
	}
	if len(summaries) != 0 {
		t.Errorf("FetchSummaries(nil) returned %d summaries, want 0", len(summaries))
	}

	matches, err := client.MatchCitations(ctx, nil)
	if err != nil {
		t.Fatalf("MatchCitations(nil) failed: %v", err)
	}
	if len(matches.Matches) != 0 {
		t.Errorf("MatchCitations(nil) returned %d matches, want 0", len(matches.Matches))
	}
}

func TestELinkRequiresAtLeastOnePMID(t *testing.T) {
	client, err := New(&Config{BaseURL: "https://example.invalid"})
	if err != nil {
		t.Fatalf("New failed: %v", err)
	}
	defer client.Close()

	ctx := context.Background()
	if _, err := client.GetRelatedArticles(ctx, nil); !errors.Is(err, ErrInvalidArgument) {
		t.Errorf("GetRelatedArticles(nil) = %v, want ErrInvalidArgument", err)
	}
	if _, err := client.GetPMCLinks(ctx, nil); !errors.Is(err, ErrInvalidArgument) {
		t.Errorf("GetPMCLinks(nil) = %v, want ErrInvalidArgument", err)
	}
	if _, err := client.GetCitations(ctx, nil); !errors.Is(err, ErrInvalidArgument) {
		t.Errorf("GetCitations(nil) = %v, want ErrInvalidArgument", err)
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

// --- Offline end-to-end coverage --------------------------------------------
//
// BaseURL points the underlying client at a local server, so these exercise the
// whole chain (Go -> cgo -> Rust -> HTTP -> XML parsing -> JSON -> Go structs)
// without touching NCBI.

func TestSearchArticlesAgainstStub(t *testing.T) {
	client := newStubClient(t, defaultStub())

	pmids, err := client.SearchArticles(context.Background(), "CRISPR", 10)
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

func TestSearchArticlesSendsTheRequestedSort(t *testing.T) {
	stub := defaultStub()
	var recorded string
	stub.observe = func(r *http.Request) {
		if pathHasSuffix(r, "/esearch.fcgi") {
			recorded = r.URL.Query().Get("sort")
		}
	}
	client := newStubClient(t, stub)

	_, err := client.SearchArticlesWithOptions(context.Background(), "CRISPR", 5,
		SearchOptions{Sort: SortPublicationDate})
	if err != nil {
		t.Fatalf("SearchArticlesWithOptions failed: %v", err)
	}
	if recorded != "pub_date" {
		t.Errorf("sort parameter = %q, want %q", recorded, "pub_date")
	}
}

func TestSearchArticlesRejectsAnUnknownSort(t *testing.T) {
	client := newStubClient(t, defaultStub())

	_, err := client.SearchArticlesWithOptions(context.Background(), "CRISPR", 5,
		SearchOptions{Sort: SortOrder("sideways")})
	if !errors.Is(err, ErrInvalidArgument) {
		t.Fatalf("SearchArticlesWithOptions with a bad sort = %v, want ErrInvalidArgument", err)
	}
}

func TestFetchArticleAgainstStub(t *testing.T) {
	client := newStubClient(t, defaultStub())

	article, err := client.FetchArticle(context.Background(), "31978945")
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
	client := newStubClient(t, defaultStub())

	articles, err := client.SearchAndFetch(context.Background(), "CRISPR", 2)
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

func TestSearchAndFetchQueryUsesTheBuildersLimitAndSort(t *testing.T) {
	stub := defaultStub()
	var recordedTerm, recordedSort, recordedRetmax string
	stub.observe = func(r *http.Request) {
		if pathHasSuffix(r, "/esearch.fcgi") {
			query := r.URL.Query()
			recordedTerm = query.Get("term")
			recordedSort = query.Get("sort")
			recordedRetmax = query.Get("retmax")
		}
	}
	client := newStubClient(t, stub)

	query := NewSearchQuery().
		TitleOrAbstract("CRISPR").
		Limit(7).
		Sort(SortPublicationDate)

	if _, err := client.SearchAndFetchQuery(context.Background(), query); err != nil {
		t.Fatalf("SearchAndFetchQuery failed: %v", err)
	}

	if recordedTerm != "CRISPR[tiab]" {
		t.Errorf("term = %q, want %q", recordedTerm, "CRISPR[tiab]")
	}
	if recordedSort != "pub_date" {
		t.Errorf("sort = %q, want %q", recordedSort, "pub_date")
	}
	if recordedRetmax != "7" {
		t.Errorf("retmax = %q, want %q", recordedRetmax, "7")
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

	_, err = client.SearchArticles(context.Background(), "CRISPR", 1)
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
	if ffiErr.Kind != KindAPI {
		t.Errorf("Kind = %q, want %q", ffiErr.Kind, KindAPI)
	}
	if ffiErr.Status != http.StatusBadRequest {
		t.Errorf("Status = %d, want %d", ffiErr.Status, http.StatusBadRequest)
	}
}

// Concurrent use must be safe: the Rust client is shared behind an Arc and the
// Go wrapper only guards against close-during-call.
func TestConcurrentCalls(t *testing.T) {
	client := newStubClient(t, defaultStub())

	const goroutines = 8
	var wg sync.WaitGroup
	errs := make(chan error, goroutines)

	for i := 0; i < goroutines; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			if _, err := client.SearchArticles(context.Background(), "CRISPR", 5); err != nil {
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

// --- Context handling --------------------------------------------------------

func TestAnAlreadyCancelledContextSkipsTheCall(t *testing.T) {
	stub := defaultStub()
	var requests int
	stub.observe = func(*http.Request) { requests++ }
	client := newStubClient(t, stub)

	ctx, cancel := context.WithCancel(context.Background())
	cancel()

	_, err := client.SearchArticles(ctx, "CRISPR", 1)
	if !errors.Is(err, context.Canceled) {
		t.Fatalf("SearchArticles with a cancelled context = %v, want context.Canceled", err)
	}
	if requests != 0 {
		t.Errorf("made %d requests with a cancelled context, want 0", requests)
	}
}

// Cancelling mid-flight must abort the request rather than wait it out. The
// stub blocks until the test releases it, so returning at all proves the
// cancellation reached the Rust side.
func TestCancellingMidCallAbortsTheRequest(t *testing.T) {
	started := make(chan struct{})
	release := make(chan struct{})

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		close(started)
		<-release
	}))
	// Ordered so the handler is released before Close waits for it; the other
	// way round deadlocks, since httptest.Server.Close joins outstanding
	// requests.
	defer server.Close()
	defer close(release)

	client, err := New(&Config{BaseURL: server.URL, RateLimit: 100})
	if err != nil {
		t.Fatalf("New failed: %v", err)
	}
	defer client.Close()

	ctx, cancel := context.WithCancel(context.Background())
	go func() {
		<-started
		cancel()
	}()

	done := make(chan error, 1)
	go func() {
		_, err := client.SearchArticles(ctx, "CRISPR", 1)
		done <- err
	}()

	select {
	case err := <-done:
		if !errors.Is(err, context.Canceled) {
			t.Fatalf("SearchArticles = %v, want context.Canceled", err)
		}
	case <-time.After(30 * time.Second):
		t.Fatal("SearchArticles did not return after its context was cancelled")
	}
}

func TestAContextDeadlineSurfacesAsDeadlineExceeded(t *testing.T) {
	release := make(chan struct{})

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		<-release
	}))
	// See TestCancellingMidCallAbortsTheRequest: release before Close.
	defer server.Close()
	defer close(release)

	client, err := New(&Config{BaseURL: server.URL, RateLimit: 100})
	if err != nil {
		t.Fatalf("New failed: %v", err)
	}
	defer client.Close()

	ctx, cancel := context.WithTimeout(context.Background(), 200*time.Millisecond)
	defer cancel()

	if _, err := client.SearchArticles(ctx, "CRISPR", 1); !errors.Is(err, context.DeadlineExceeded) {
		t.Fatalf("SearchArticles = %v, want context.DeadlineExceeded", err)
	}
}

// Every completed call must release its token and join its watchdog, so a long
// run of cancellable calls must not leak goroutines.
func TestSuccessfulCallsDoNotLeakWatchdogs(t *testing.T) {
	client := newStubClient(t, defaultStub())

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	for i := 0; i < 50; i++ {
		if _, err := client.SearchArticles(ctx, "CRISPR", 1); err != nil {
			t.Fatalf("call %d failed: %v", i, err)
		}
	}

	// The watchdogs are joined inside each call, so nothing should still be
	// waiting on ctx by the time the loop finishes.
	select {
	case <-ctx.Done():
		t.Fatal("context was cancelled unexpectedly")
	default:
	}
}
