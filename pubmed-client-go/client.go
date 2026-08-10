// Package pubmedclient provides Go bindings for the PubMed and PMC (PubMed
// Central) APIs, backed by the Rust `pubmed-client` crate through cgo.
//
// The surface is intentionally small (an MVP, mirroring the R bindings):
// search PubMed, fetch article metadata, and retrieve PMC full text or
// Markdown.
//
//	client, err := pubmedclient.New(&pubmedclient.Config{
//		Email: "you@example.com",
//		Tool:  "my-app",
//	})
//	if err != nil {
//		return err
//	}
//	defer client.Close()
//
//	articles, err := client.SearchAndFetch("CRISPR gene editing", 5)
//
// Building this package requires the Rust static library; see the README and
// the Makefile in this directory.
//
// # Rate limits
//
// NCBI limits unauthenticated clients to 3 requests/second (10 with an API
// key). The underlying client enforces this with a shared token bucket, so
// concurrent calls from many goroutines stay within the limit.
package pubmedclient

import (
	"encoding/json"
	"fmt"
	"runtime"
	"sync"
	"time"
)

// Config holds optional client settings. The zero value is valid and selects
// the library defaults throughout.
type Config struct {
	// APIKey is an NCBI API key, which raises the rate limit from 3 to 10
	// requests per second.
	APIKey string
	// Email and Tool identify the caller to NCBI. Providing both is
	// recommended, and NCBI asks for them on high-volume use.
	Email string
	Tool  string
	// RateLimit overrides the requests-per-second limit. Zero keeps the
	// default implied by APIKey.
	RateLimit float64
	// Timeout bounds each HTTP request. Zero keeps the default. Resolution is
	// one second; shorter non-zero values are rounded up.
	Timeout time.Duration
	// UserAgent overrides the HTTP User-Agent header.
	UserAgent string
	// BaseURL overrides the NCBI E-utilities base URL. Mainly useful for
	// pointing tests at a local server.
	BaseURL string
	// Cache enables the in-memory response cache.
	Cache bool
}

// configPayload is the wire form of [Config]. Pointer fields distinguish
// "unset" from "explicitly zero"; the Rust side rejects unknown keys.
type configPayload struct {
	APIKey         *string  `json:"api_key,omitempty"`
	Email          *string  `json:"email,omitempty"`
	Tool           *string  `json:"tool,omitempty"`
	RateLimit      *float64 `json:"rate_limit,omitempty"`
	TimeoutSeconds *uint64  `json:"timeout_seconds,omitempty"`
	UserAgent      *string  `json:"user_agent,omitempty"`
	BaseURL        *string  `json:"base_url,omitempty"`
	Cache          bool     `json:"cache"`
}

func (c *Config) marshal() (string, error) {
	if c == nil {
		return "", nil
	}

	payload := configPayload{Cache: c.Cache}
	if c.APIKey != "" {
		payload.APIKey = &c.APIKey
	}
	if c.Email != "" {
		payload.Email = &c.Email
	}
	if c.Tool != "" {
		payload.Tool = &c.Tool
	}
	if c.RateLimit != 0 {
		payload.RateLimit = &c.RateLimit
	}
	if c.Timeout != 0 {
		// Round up so a sub-second timeout never becomes "no timeout".
		seconds := uint64((c.Timeout + time.Second - 1) / time.Second)
		payload.TimeoutSeconds = &seconds
	}
	if c.UserAgent != "" {
		payload.UserAgent = &c.UserAgent
	}
	if c.BaseURL != "" {
		payload.BaseURL = &c.BaseURL
	}

	encoded, err := json.Marshal(payload)
	if err != nil {
		return "", &Error{Op: "New", Message: "failed to encode config: " + err.Error()}
	}
	return string(encoded), nil
}

// Client is a PubMed and PMC client. It is safe for concurrent use by multiple
// goroutines; calls block until the request completes.
//
// A Client owns memory outside Go's heap, so it must be released with
// [Client.Close] when no longer needed.
type Client struct {
	// mu guards against a call running while Close frees the handle. Calls take
	// it for reading and so do not serialize with each other.
	mu     sync.RWMutex
	handle handle
	closed bool
}

// New creates a client. Passing a nil Config selects the library defaults.
func New(config *Config) (*Client, error) {
	configJSON, err := config.marshal()
	if err != nil {
		return nil, err
	}

	h, err := newHandle(configJSON)
	if err != nil {
		return nil, err
	}

	client := &Client{handle: h}
	// Safety net for callers who forget Close; Close clears it.
	runtime.SetFinalizer(client, (*Client).Close)
	return client, nil
}

// Close releases the underlying Rust client. It is idempotent, and every
// subsequent call on the Client returns [ErrClosed].
func (c *Client) Close() error {
	c.mu.Lock()
	defer c.mu.Unlock()

	if c.closed {
		return nil
	}
	c.closed = true
	freeHandle(c.handle)
	c.handle = nil
	runtime.SetFinalizer(c, nil)
	return nil
}

// call runs fn with the live handle, refusing to touch a closed client.
func (c *Client) call(fn func(handle) (string, error)) (string, error) {
	c.mu.RLock()
	defer c.mu.RUnlock()

	if c.closed {
		return "", ErrClosed
	}
	return fn(c.handle)
}

// decode parses a JSON response from the Rust side into target.
func decode(op, raw string, target any) error {
	if err := json.Unmarshal([]byte(raw), target); err != nil {
		return &Error{Op: op, Message: "failed to decode response: " + err.Error()}
	}
	return nil
}

// checkLimit rejects limits the Rust side would reject anyway, but with a
// clearer message and without a round trip.
func checkLimit(op string, limit int) error {
	if limit <= 0 {
		return &Error{Op: op, Message: fmt.Sprintf("limit must be positive, got %d", limit)}
	}
	return nil
}

// SearchArticles searches PubMed and returns up to limit matching PMIDs.
//
// The query accepts PubMed's full syntax, including field tags such as
// "cancer[ti] AND 2023[pdat]".
func (c *Client) SearchArticles(query string, limit int) ([]string, error) {
	if err := checkLimit("SearchArticles", limit); err != nil {
		return nil, err
	}

	raw, err := c.call(func(h handle) (string, error) {
		return ffiSearchArticles(h, query, limit)
	})
	if err != nil {
		return nil, err
	}

	var pmids []string
	if err := decode("SearchArticles", raw, &pmids); err != nil {
		return nil, err
	}
	return pmids, nil
}

// FetchArticle fetches the full metadata for a single PMID.
func (c *Client) FetchArticle(pmid string) (*Article, error) {
	raw, err := c.call(func(h handle) (string, error) {
		return ffiFetchArticle(h, pmid)
	})
	if err != nil {
		return nil, err
	}

	var article Article
	if err := decode("FetchArticle", raw, &article); err != nil {
		return nil, err
	}
	return &article, nil
}

// FetchArticles fetches metadata for several PMIDs in one batched request.
// Passing no PMIDs returns an empty slice without contacting NCBI.
func (c *Client) FetchArticles(pmids []string) ([]Article, error) {
	if len(pmids) == 0 {
		return []Article{}, nil
	}

	encoded, err := json.Marshal(pmids)
	if err != nil {
		return nil, &Error{Op: "FetchArticles", Message: "failed to encode pmids: " + err.Error()}
	}

	raw, err := c.call(func(h handle) (string, error) {
		return ffiFetchArticles(h, string(encoded))
	})
	if err != nil {
		return nil, err
	}

	var articles []Article
	if err := decode("FetchArticles", raw, &articles); err != nil {
		return nil, err
	}
	return articles, nil
}

// SearchAndFetch searches PubMed and fetches metadata for each hit, combining
// [Client.SearchArticles] and [Client.FetchArticles] into one call.
func (c *Client) SearchAndFetch(query string, limit int) ([]Article, error) {
	if err := checkLimit("SearchAndFetch", limit); err != nil {
		return nil, err
	}

	raw, err := c.call(func(h handle) (string, error) {
		return ffiSearchAndFetch(h, query, limit)
	})
	if err != nil {
		return nil, err
	}

	var articles []Article
	if err := decode("SearchAndFetch", raw, &articles); err != nil {
		return nil, err
	}
	return articles, nil
}

// FetchFullText retrieves the full text of a PMC article. The pmcid may be
// given with or without the "PMC" prefix.
//
// Full text is only available for articles in the PMC Open Access subset; use
// [Client.CheckPMCAvailability] to test a PMID first.
func (c *Client) FetchFullText(pmcid string) (*PMCArticle, error) {
	raw, err := c.call(func(h handle) (string, error) {
		return ffiFetchFullText(h, pmcid)
	})
	if err != nil {
		return nil, err
	}

	var article PMCArticle
	if err := decode("FetchFullText", raw, &article); err != nil {
		return nil, err
	}
	return &article, nil
}

// FetchMarkdown retrieves a PMC article and renders it as Markdown.
func (c *Client) FetchMarkdown(pmcid string) (string, error) {
	return c.call(func(h handle) (string, error) {
		return ffiFetchMarkdown(h, pmcid)
	})
}

// CheckPMCAvailability reports whether a PMID has PMC full text available,
// returning the PMCID when it does.
func (c *Client) CheckPMCAvailability(pmid string) (pmcid string, available bool, err error) {
	raw, err := c.call(func(h handle) (string, error) {
		return ffiCheckPMCAvailability(h, pmid)
	})
	if err != nil {
		return "", false, err
	}

	// JSON `null` when unavailable, otherwise the PMCID as a JSON string.
	var result *string
	if err := decode("CheckPMCAvailability", raw, &result); err != nil {
		return "", false, err
	}
	if result == nil {
		return "", false, nil
	}
	return *result, true, nil
}
