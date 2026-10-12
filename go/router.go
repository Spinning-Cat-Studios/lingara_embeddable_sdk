package lingaraembed

import (
	"context"
	"errors"
	"fmt"
	"log/slog"
	"net/http"

	lingara "github.com/Spinning-Cat-Studios/lingara_api_clients/go"
)

// RoutedEvent is a verified event and the player it concerns. PlayerRef is
// "" and Known false for a subject the directory does not know: your own
// subject, or a player you deleted. Event is the library's union, a
// lingara.UnknownEvent included.
type RoutedEvent struct {
	Event     lingara.Event
	PlayerRef string
	Known     bool
}

// HandlerError is a failure of Handle's onEvent, as onError receives it.
type HandlerError struct {
	Err error
}

func (e *HandlerError) Error() string {
	return "lingaraembed: the event handler failed: " + e.Err.Error()
}

// Unwrap returns the handler's own error.
func (e *HandlerError) Unwrap() error { return e.Err }

// PlayerRouter verifies Lingara's webhooks and resolves each to a player
// (ADR 1.10.26af D8). It is safe for concurrent use.
type PlayerRouter struct {
	webhook   *lingara.Webhook
	directory PlayerDirectory
	onError   func(context.Context, error)
}

// NewPlayerRouter builds the library's verifier from your webhook secret,
// or both during a rotation, once: a malformed secret fails here, at
// startup, with the library's error.
func NewPlayerRouter(d PlayerDirectory, secrets ...string) (*PlayerRouter, error) {
	if d == nil {
		return nil, errors.New("lingaraembed: NewPlayerRouter needs a PlayerDirectory")
	}
	webhook, err := lingara.NewWebhook(secrets...)
	if err != nil {
		return nil, err
	}
	return &PlayerRouter{webhook: webhook, directory: d, onError: logError}, nil
}

// logError is onError's default: log/slog's default logger.
func logError(ctx context.Context, err error) {
	slog.ErrorContext(ctx, "lingara webhook: answered 500", "error", err.Error())
}

// WithOnError returns a copy of r whose Handle sends a 500's cause, a
// *DirectoryError or a *HandlerError, to onError rather than to log/slog's
// default logger. The error never carries the body, a signature or a secret.
func (r *PlayerRouter) WithOnError(onError func(context.Context, error)) *PlayerRouter {
	copied := *r
	copied.onError = onError
	return &copied
}

// Route verifies the raw body, then looks the event's subject up. A failed
// verification is the library's *lingara.WebhookVerificationError, returned
// before the directory is asked, so a forged event never reaches your store.
// A failed lookup is a *DirectoryError.
func (r *PlayerRouter) Route(ctx context.Context, body []byte, h http.Header) (RoutedEvent, error) {
	event, err := r.webhook.Verify(body, h)
	if err != nil {
		return RoutedEvent{}, err
	}
	playerRef, known, err := r.directory.PlayerRefFor(ctx, event.Meta().Subject)
	if err != nil {
		return RoutedEvent{}, &DirectoryError{Err: err}
	}
	return RoutedEvent{Event: event, PlayerRef: playerRef, Known: known}, nil
}

// Handle is Route, then onEvent, answered as the status your route sends
// with an empty body: 204 handled, 401 a bad signature, 400 a valid
// signature over a malformed payload, 500 the directory or onEvent failed
// (the cause goes to onError). Lingara redelivers every non-2xx. onEvent runs
// inline, and delivery expects an answer within 10 s: enqueue slow work.
// Deduplicate on the event's ID: delivery is at least once.
func (r *PlayerRouter) Handle(ctx context.Context, body []byte, h http.Header, onEvent func(context.Context, RoutedEvent) error) int {
	routed, err := r.Route(ctx, body, h)
	var refused *lingara.WebhookVerificationError
	switch {
	case errors.As(err, &refused) && refused.Reason == lingara.MalformedPayload:
		return http.StatusBadRequest
	case errors.As(err, &refused):
		return http.StatusUnauthorized
	case err != nil:
		return r.failed(ctx, err)
	}
	if err := onEvent(ctx, routed); err != nil {
		return r.failed(ctx, &HandlerError{Err: err})
	}
	return http.StatusNoContent
}

func (r *PlayerRouter) failed(ctx context.Context, err error) int {
	r.onError(ctx, err)
	return http.StatusInternalServerError
}

// Format renders the verifier through its own redacting Format.
func (r *PlayerRouter) Format(f fmt.State, _ rune) {
	_, _ = fmt.Fprintf(f, "lingaraembed.PlayerRouter(%v)", r.webhook)
}

// LogValue is the verifier's own redacted group.
func (r *PlayerRouter) LogValue() slog.Value {
	return slog.GroupValue(slog.Any("webhook", r.webhook))
}
