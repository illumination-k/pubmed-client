package pubmedclient

import (
	"errors"
	"testing"
)

func TestErrorMessageIncludesOp(t *testing.T) {
	err := &Error{Op: "FetchArticle", Message: "boom"}
	if got, want := err.Error(), "pubmedclient.FetchArticle: boom"; got != want {
		t.Errorf("Error() = %q, want %q", got, want)
	}
}

func TestParseErrorReadsTheEnvelope(t *testing.T) {
	err := parseError("FetchArticle",
		`{"kind":"article_not_found","message":"Article not found: PMID 1"}`)

	var typed *Error
	if !errors.As(err, &typed) {
		t.Fatalf("parseError returned %T, want *Error", err)
	}
	if typed.Kind != KindArticleNotFound {
		t.Errorf("Kind = %q, want %q", typed.Kind, KindArticleNotFound)
	}
	if typed.Message != "Article not found: PMID 1" {
		t.Errorf("Message = %q", typed.Message)
	}
	if typed.Op != "FetchArticle" {
		t.Errorf("Op = %q", typed.Op)
	}
}

func TestParseErrorReadsTheApiStatus(t *testing.T) {
	err := parseError("SearchArticles",
		`{"kind":"api","message":"API error 429: slow down","status":429}`)

	var typed *Error
	if !errors.As(err, &typed) {
		t.Fatalf("parseError returned %T, want *Error", err)
	}
	if typed.Status != 429 {
		t.Errorf("Status = %d, want 429", typed.Status)
	}
	// A 429 is how NCBI reports throttling, so it must match the rate-limit
	// sentinel even though its kind is "api".
	if !errors.Is(err, ErrRateLimited) {
		t.Error("a 429 does not match ErrRateLimited")
	}
}

// A payload that is not an envelope must survive as the message rather than be
// dropped, so a future Rust build cannot silently lose error text.
func TestParseErrorKeepsANonEnvelopePayload(t *testing.T) {
	err := parseError("FetchArticle", "something went wrong")

	var typed *Error
	if !errors.As(err, &typed) {
		t.Fatalf("parseError returned %T, want *Error", err)
	}
	if typed.Kind != KindUnknown {
		t.Errorf("Kind = %q, want %q", typed.Kind, KindUnknown)
	}
	if typed.Message != "something went wrong" {
		t.Errorf("Message = %q", typed.Message)
	}
}

// An unrecognised kind must arrive verbatim rather than be flattened to
// unknown, so a newer Rust build stays inspectable from an older Go build.
func TestParseErrorKeepsAnUnrecognizedKind(t *testing.T) {
	err := parseError("Whatever", `{"kind":"from_the_future","message":"hello"}`)

	var typed *Error
	if !errors.As(err, &typed) {
		t.Fatalf("parseError returned %T, want *Error", err)
	}
	if typed.Kind != Kind("from_the_future") {
		t.Errorf("Kind = %q, want the kind as sent", typed.Kind)
	}
	// It matches no sentinel, which is the safe default.
	for _, sentinel := range []error{ErrNotFound, ErrRateLimited, ErrInvalidQuery} {
		if errors.Is(err, sentinel) {
			t.Errorf("an unknown kind matched %v", sentinel)
		}
	}
}

func TestSentinelMapping(t *testing.T) {
	tests := []struct {
		kind     Kind
		sentinel error
	}{
		{KindInvalidArgument, ErrInvalidArgument},
		{KindArticleNotFound, ErrNotFound},
		{KindPMCNotAvailable, ErrPMCNotAvailable},
		{KindRateLimit, ErrRateLimited},
		{KindInvalidQuery, ErrInvalidQuery},
		{KindSearchLimitExceeded, ErrInvalidQuery},
	}

	for _, test := range tests {
		err := error(&Error{Op: "Test", Kind: test.kind, Message: "m"})
		if !errors.Is(err, test.sentinel) {
			t.Errorf("kind %q does not match %v", test.kind, test.sentinel)
		}
	}
}

func TestSentinelsDoNotOverlap(t *testing.T) {
	err := error(&Error{Op: "Test", Kind: KindArticleNotFound, Message: "m"})

	for _, sentinel := range []error{ErrPMCNotAvailable, ErrRateLimited, ErrInvalidQuery, ErrInvalidArgument} {
		if errors.Is(err, sentinel) {
			t.Errorf("article_not_found also matched %v", sentinel)
		}
	}
	if errors.Is(err, ErrClosed) {
		t.Error("article_not_found matched ErrClosed")
	}
}
