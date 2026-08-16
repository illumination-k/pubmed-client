// Package pubmedclient provides Go bindings for the PubMed and PMC (PubMed
// Central) APIs, backed by the Rust `pubmed-client` crate through cgo.
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
//	articles, err := client.SearchAndFetch(ctx, "CRISPR gene editing", 5)
//
// The surface covers PubMed search and metadata (ESearch, EFetch, ESummary),
// the discovery APIs (ELink, EInfo, EGQuery, ECitMatch, ESpell), PMC full text,
// XML, Markdown and Open Access downloads, a query builder, and citation
// export. Building this package requires the Rust static library; see the
// README.
//
// # Contexts
//
// Every call takes a [context.Context] and honours cancellation: a cancelled
// context aborts the in-flight HTTP request rather than merely reporting the
// cancellation afterwards, and the call returns the context's error.
//
// # Rate limits
//
// NCBI limits unauthenticated clients to 3 requests/second (10 with an API
// key). The underlying client enforces this with a shared token bucket, so
// concurrent calls from many goroutines stay within the limit.
//
// # Errors
//
// Failures arrive as [*Error], carrying the failing operation, a [Kind], and a
// message. The common causes also match the package sentinels:
//
//	if errors.Is(err, pubmedclient.ErrPMCNotAvailable) {
//		// expected for articles outside the PMC Open Access subset
//	}
package pubmedclient

import (
	"context"
	"encoding/json"
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
		return "", encodeError("New", "config", err)
	}
	return string(encoded), nil
}

// Client is a PubMed and PMC client. It is safe for concurrent use by multiple
// goroutines; calls block until the request completes or the context is done.
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
//
// Close blocks until any in-flight call has finished, so cancelling a context
// and closing immediately is safe.
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

// call runs fn with the live handle and a cancellation token wired to ctx.
//
// The token is what makes cancellation real rather than advisory: a watchdog
// goroutine fires it when ctx is done, the Rust side drops the request future,
// and the blocked call returns promptly. A context that can never be cancelled
// (such as [context.Background]) skips the token and the goroutine entirely.
func (c *Client) call(ctx context.Context, op string, fn func(handle, token) (string, error)) (string, error) {
	if err := ctx.Err(); err != nil {
		return "", err
	}

	// Held for the whole call, which is why Close waits for callers to finish
	// rather than freeing the handle out from under them.
	c.mu.RLock()
	defer c.mu.RUnlock()

	if c.closed {
		return "", ErrClosed
	}

	done := ctx.Done()
	if done == nil {
		return fn(c.handle, nil)
	}

	cancel := newToken()
	stop := make(chan struct{})
	watchdogDone := make(chan struct{})
	go func() {
		defer close(watchdogDone)
		select {
		case <-done:
			triggerToken(cancel)
		case <-stop:
		}
	}()
	// Join the watchdog before freeing: it may be mid-select, and triggering a
	// freed token would be a use-after-free.
	defer func() {
		close(stop)
		<-watchdogDone
		freeToken(cancel)
	}()

	raw, err := fn(c.handle, cancel)
	if err != nil {
		// Report the context's own error, so errors.Is(err, context.Canceled)
		// and context.DeadlineExceeded work as callers expect.
		if ctxErr := ctx.Err(); ctxErr != nil {
			return "", ctxErr
		}
		return "", err
	}
	return raw, nil
}

// decode parses a JSON response from the Rust side into target.
func decode(op, raw string, target any) error {
	if err := json.Unmarshal([]byte(raw), target); err != nil {
		return decodeError(op, err)
	}
	return nil
}

// callJSON runs a call and decodes its JSON response into target.
func (c *Client) callJSON(ctx context.Context, op string, target any, fn func(handle, token) (string, error)) error {
	raw, err := c.call(ctx, op, fn)
	if err != nil {
		return err
	}
	return decode(op, raw, target)
}

// checkLimit rejects limits the Rust side would reject anyway, but with a
// clearer message and without a round trip.
func checkLimit(op string, limit int) error {
	if limit <= 0 {
		return limitError(op, limit)
	}
	return nil
}

// marshalArg encodes a call argument that crosses the boundary as JSON.
func marshalArg(op, what string, value any) (string, error) {
	encoded, err := json.Marshal(value)
	if err != nil {
		return "", encodeError(op, what, err)
	}
	return string(encoded), nil
}
