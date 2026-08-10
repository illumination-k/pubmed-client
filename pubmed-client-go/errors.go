package pubmedclient

import (
	"encoding/json"
	"errors"
	"strconv"
)

// Kind classifies a failure reported by the underlying Rust client, so callers
// can branch on the cause without matching on message text.
//
// Prefer the sentinel errors below with [errors.Is] for the common cases; Kind
// is there for the rest.
type Kind string

const (
	// KindUnknown is used when the Rust side reported a failure this package
	// could not classify — including a future kind an older build of this
	// package does not know about.
	KindUnknown Kind = "unknown"
	// KindInvalidArgument means an argument was rejected before any request was
	// made.
	KindInvalidArgument Kind = "invalid_argument"
	// KindCancelled means the call was aborted through its context. Calls
	// translate this into the context's own error, so it rarely surfaces.
	KindCancelled Kind = "cancelled"
	// KindRequest means the HTTP request itself failed: DNS, connect, timeout,
	// or TLS.
	KindRequest Kind = "request"
	// KindAPI means NCBI answered with a non-success status, carried in
	// [Error.Status].
	KindAPI Kind = "api"
	// KindRateLimit means the request was rejected as too frequent.
	KindRateLimit Kind = "rate_limit"
	// KindInvalidQuery means the query was malformed or empty.
	KindInvalidQuery Kind = "invalid_query"
	// KindSearchLimitExceeded means more results were requested than PubMed can
	// return (9,999).
	KindSearchLimitExceeded Kind = "search_limit_exceeded"
	// KindHistorySession means a WebEnv session expired or was rejected.
	KindHistorySession Kind = "history_session"
	// KindWebEnvUnavailable means a history-server operation ran without a
	// session.
	KindWebEnvUnavailable Kind = "webenv_unavailable"
	// KindArticleNotFound means PubMed has no article for the requested PMID.
	KindArticleNotFound Kind = "article_not_found"
	// KindPMCNotAvailable means the article has no PMC full text.
	KindPMCNotAvailable Kind = "pmc_not_available"
	// KindInvalidPMID means the PMID was not in a valid format.
	KindInvalidPMID Kind = "invalid_pmid"
	// KindInvalidPMCID means the PMCID was not in a valid format.
	KindInvalidPMCID Kind = "invalid_pmcid"
	// KindXMLParse means the response was not valid XML.
	KindXMLParse Kind = "xml_parse"
	// KindJSONParse means the response was not valid JSON.
	KindJSONParse Kind = "json_parse"
	// KindIO means a filesystem operation failed.
	KindIO Kind = "io"
	// KindPanic means a panic was caught at the FFI boundary; always a bug in
	// the bindings.
	KindPanic Kind = "panic"
	// KindInternal covers anything else, including decode failures in this
	// package.
	KindInternal Kind = "internal"
)

// Error is returned when a call into the underlying Rust client fails.
//
// Op names the Go method that failed, Kind classifies the cause, and Message
// carries the description produced by `pubmed-client`.
type Error struct {
	// Op is the Go method that failed, e.g. "FetchArticle".
	Op string
	// Kind classifies the failure. Match it with the sentinel errors below
	// through [errors.Is] where one exists.
	Kind Kind
	// Message describes the failure.
	Message string
	// Status is the HTTP status NCBI returned. It is set only when Kind is
	// [KindAPI].
	Status int
}

func (e *Error) Error() string {
	return "pubmedclient." + e.Op + ": " + e.Message
}

// Is reports whether this error matches one of the package sentinels, so
// callers can write errors.Is(err, pubmedclient.ErrNotFound) instead of
// inspecting [Error.Kind].
func (e *Error) Is(target error) bool {
	switch target {
	case ErrInvalidArgument:
		return e.Kind == KindInvalidArgument
	case ErrNotFound:
		return e.Kind == KindArticleNotFound
	case ErrPMCNotAvailable:
		return e.Kind == KindPMCNotAvailable
	case ErrRateLimited:
		// NCBI reports throttling as 429 rather than through a dedicated
		// error, so both spellings map to the same sentinel.
		return e.Kind == KindRateLimit || (e.Kind == KindAPI && e.Status == 429)
	case ErrInvalidQuery:
		return e.Kind == KindInvalidQuery || e.Kind == KindSearchLimitExceeded
	default:
		return false
	}
}

// Sentinel errors for the failures worth branching on. Match them with
// [errors.Is]; the concrete error is always an [*Error] carrying the details.
var (
	// ErrClosed is returned by every method of a [Client] that has been closed.
	ErrClosed = errors.New("pubmedclient: client is closed")
	// ErrInvalidArgument reports an argument rejected before any request was
	// made.
	ErrInvalidArgument = errors.New("pubmedclient: invalid argument")
	// ErrNotFound reports that PubMed has no article for the requested PMID.
	ErrNotFound = errors.New("pubmedclient: article not found")
	// ErrPMCNotAvailable reports that an article has no PMC full text. It is
	// the expected outcome for the majority of PubMed articles, which are not
	// in the PMC Open Access subset.
	ErrPMCNotAvailable = errors.New("pubmedclient: PMC full text not available")
	// ErrRateLimited reports that NCBI rejected the request as too frequent.
	// Setting Config.APIKey raises the limit from 3 to 10 requests per second.
	ErrRateLimited = errors.New("pubmedclient: rate limit exceeded")
	// ErrInvalidQuery reports a malformed, empty, or over-long query.
	ErrInvalidQuery = errors.New("pubmedclient: invalid query")
)

// errorEnvelope is the JSON the Rust side writes to out_err.
type errorEnvelope struct {
	Kind    Kind   `json:"kind"`
	Message string `json:"message"`
	Status  int    `json:"status"`
}

// parseError turns an error envelope into an [*Error].
//
// A payload that is not an envelope is kept verbatim as the message under
// [KindUnknown], so a future Rust build that changes the format degrades to the
// pre-envelope behaviour rather than losing the message.
func parseError(op, payload string) error {
	var envelope errorEnvelope
	if err := json.Unmarshal([]byte(payload), &envelope); err != nil || envelope.Message == "" {
		return &Error{Op: op, Kind: KindUnknown, Message: payload}
	}
	return &Error{
		Op:      op,
		Kind:    envelope.Kind,
		Message: envelope.Message,
		Status:  envelope.Status,
	}
}

// decodeError reports a response this package could not decode. It means the
// Rust side and models.go have drifted apart, so it is deliberately loud about
// what failed.
func decodeError(op string, cause error) error {
	return &Error{
		Op:      op,
		Kind:    KindInternal,
		Message: "failed to decode response: " + cause.Error(),
	}
}

// encodeError reports an argument this package could not encode.
func encodeError(op, what string, cause error) error {
	return &Error{
		Op:      op,
		Kind:    KindInternal,
		Message: "failed to encode " + what + ": " + cause.Error(),
	}
}

// argError reports an argument rejected locally, without a round trip.
func argError(op, message string) error {
	return &Error{Op: op, Kind: KindInvalidArgument, Message: message}
}

// limitError is the shared message for a non-positive result limit.
func limitError(op string, limit int) error {
	return argError(op, "limit must be positive, got "+strconv.Itoa(limit))
}
