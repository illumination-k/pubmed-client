package pubmedclient

import "errors"

// Error is returned when a call into the underlying Rust client fails. Op names
// the Go method that failed and Message carries the message produced by
// `pubmed-client` (an HTTP failure, a parse failure, an invalid argument, …).
type Error struct {
	Op      string
	Message string
}

func (e *Error) Error() string {
	return "pubmedclient." + e.Op + ": " + e.Message
}

// ErrClosed is returned by every method of a [Client] that has been closed.
var ErrClosed = errors.New("pubmedclient: client is closed")
