package lingaraembed

import (
	"context"
	"errors"
	"net/http"
	"strings"
	"testing"
	"time"
)

type signatureVector struct {
	Name    string            `json:"name"`
	Secrets []string          `json:"secrets"`
	Headers map[string]string `json:"headers"`
	Body    string            `json:"body"`
	Now     int64             `json:"now"`
}

// signed returns E7's vector name: its body, its headers, and a router over
// its secrets reading the time at the vector's own now through the
// library's clock seam.
func signed(t *testing.T, name string, d PlayerDirectory) (*PlayerRouter, []byte, http.Header) {
	t.Helper()
	var all struct {
		Vectors []signatureVector `json:"vectors"`
	}
	readVectors(t, "webhook-signatures.json", &all)
	for _, v := range all.Vectors {
		if v.Name != name {
			continue
		}
		r, err := NewPlayerRouter(d, v.Secrets...)
		if err != nil {
			t.Fatal(err)
		}
		r.webhook = r.webhook.WithClock(func() time.Time { return time.Unix(v.Now, 0) })
		h := http.Header{}
		for k, value := range v.Headers {
			h.Set(k, value)
		}
		return r, []byte(v.Body), h
	}
	t.Fatalf("no vector %s", name)
	return nil, nil, nil
}

// counting is a MemoryDirectory that counts lookups, and fails them when
// asked to.
type counting struct {
	*MemoryDirectory
	lookups int
	fail    bool
}

func (c *counting) PlayerRefFor(ctx context.Context, subject string) (string, bool, error) {
	c.lookups++
	if c.fail {
		return "", false, errors.New("the store is down")
	}
	return c.MemoryDirectory.PlayerRefFor(ctx, subject)
}

func knowing(fail bool) *counting {
	d := &counting{MemoryDirectory: NewMemoryDirectory(), fail: fail}
	_ = d.Remember(context.Background(), "p1", "lgr_sub_conformance")
	return d
}

// recorder is an onEvent that records what it saw, and an onError that
// records what it was given.
type recorder struct {
	seen   []RoutedEvent
	errors []string
}

func (r *recorder) onEvent(_ context.Context, routed RoutedEvent) error {
	r.seen = append(r.seen, routed)
	return nil
}

func (r *recorder) onError(_ context.Context, err error) { r.errors = append(r.errors, err.Error()) }

func expectStatus(t *testing.T, label string, got, want int) {
	t.Helper()
	if got != want {
		t.Fatalf("%s: status %d, want %d", label, got, want)
	}
}

// expectRouted checks the last event onEvent saw.
func expectRouted(t *testing.T, rec *recorder, playerRef string, known bool) {
	t.Helper()
	if len(rec.seen) == 0 {
		t.Fatal("onEvent was never called")
	}
	got := rec.seen[len(rec.seen)-1]
	if got.PlayerRef != playerRef || got.Known != known || got.Event.Meta().ID != "lgr_evt_conformance1" {
		t.Fatalf("routed %+v, want player %q (known %t)", got, playerRef, known)
	}
}

// expectErrors checks what onError was given, in order.
func expectErrors(t *testing.T, rec *recorder, want ...string) {
	t.Helper()
	if strings.Join(rec.errors, "\n") != strings.Join(want, "\n") {
		t.Fatalf("onError saw %q, want %q", rec.errors, want)
	}
}

// TestHandleVerifiesBeforeResolving: 1.10.26af AC14. Through Handle, a
// tampered body is 401 and never reaches the directory or onEvent; a valid
// event reaches onEvent with the remembered player_ref; an unknown subject
// arrives with none; a raising handler and a raising directory are each 500
// and reach onError; a valid signature over a malformed payload is 400.
func TestHandleVerifiesBeforeResolving(t *testing.T) {
	ctx, rec := context.Background(), &recorder{}

	dir := knowing(false)
	r, body, h := signed(t, "tampered-body", dir)
	expectStatus(t, "tampered", r.Handle(ctx, body, h, rec.onEvent), http.StatusUnauthorized)
	if dir.lookups+len(rec.seen) != 0 {
		t.Fatalf("a forged event reached the directory (%d) or onEvent (%d)", dir.lookups, len(rec.seen))
	}

	r, body, h = signed(t, "valid-single-signature", dir)
	expectStatus(t, "valid", r.Handle(ctx, body, h, rec.onEvent), http.StatusNoContent)
	expectRouted(t, rec, "p1", true)
	_ = dir.Forget(ctx, "p1")
	expectStatus(t, "unknown subject", r.Handle(ctx, body, h, rec.onEvent), http.StatusNoContent)
	expectRouted(t, rec, "", false)

	r, _, _ = signed(t, "valid-single-signature", knowing(false))
	raise := func(context.Context, RoutedEvent) error { return errors.New("the handler broke") }
	expectStatus(t, "handler raises", r.WithOnError(rec.onError).Handle(ctx, body, h, raise), http.StatusInternalServerError)
	r, _, _ = signed(t, "valid-single-signature", knowing(true))
	expectStatus(t, "directory raises", r.WithOnError(rec.onError).Handle(ctx, body, h, rec.onEvent), http.StatusInternalServerError)
	expectErrors(t, rec, "lingaraembed: the event handler failed: the handler broke",
		"lingaraembed: the player directory failed: the store is down")
	if len(rec.seen) != 2 {
		t.Fatalf("onEvent saw %d events, want 2: a failed lookup never calls it", len(rec.seen))
	}

	r, body, h = signed(t, "body-not-json", knowing(false))
	expectStatus(t, "malformed payload", r.Handle(ctx, body, h, rec.onEvent), http.StatusBadRequest)
}
