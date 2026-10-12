package main

import (
	"context"
	"errors"
	"fmt"
	"io"
	"net/http"
	"sync"

	lingara "github.com/Spinning-Cat-Studios/lingara_api_clients/go"
	lingaraembed "github.com/Spinning-Cat-Studios/lingara_embeddable_sdk/go"
)

// seen is what one delivery's onEvent saw.
type seen struct {
	PlayerRef *string
	EventID   string
	Unknown   bool
}

// armed is the case the receiver is answering.
type armed struct {
	deliver   []deliverItem
	directory *recording
	seen      []*seen
	received  int
	answered  int
}

type deliverItem struct {
	Handler string `json:"handler"`
	Router  *struct {
		Secrets []string `json:"secrets"`
	} `json:"router"`
	Expect struct {
		Handler any `json:"handler"`
	} `json:"expect"`
}

// receiver is the webhook receiver the conformance server posts to.
// Delivery n of the armed case is answered through a PlayerRouter built
// from that item's router.secrets, whose onEvent records what it saw and
// raises when the item's handler is raise.
type receiver struct {
	mu sync.Mutex
	// armed is nil between cases.
	armed *armed
	// secrets are primary and secondary, from LINGARA_EMBED_WEBHOOK_SECRETS.
	secrets map[string]string
	sink    *sink
}

// router is the PlayerRouter delivery item is answered through.
func (rc *receiver) router(item deliverItem, d lingaraembed.PlayerDirectory) (*lingaraembed.PlayerRouter, error) {
	names := []string{"primary"}
	if item.Router != nil {
		names = item.Router.Secrets
	}
	secrets := make([]string, 0, len(names))
	for _, name := range names {
		secret, ok := rc.secrets[name]
		if !ok {
			return nil, fmt.Errorf("no secret named %s", name)
		}
		secrets = append(secrets, secret)
	}
	r, err := lingaraembed.NewPlayerRouter(d, secrets...)
	if err != nil {
		return nil, err
	}
	return r.WithOnError(func(_ context.Context, err error) { rc.sink.push("on_error: " + err.Error()) }), nil
}

func (rc *receiver) ServeHTTP(w http.ResponseWriter, req *http.Request) {
	body, _ := io.ReadAll(req.Body)
	rc.mu.Lock()
	a := rc.armed
	if a == nil {
		rc.mu.Unlock()
		w.WriteHeader(http.StatusConflict)
		return
	}
	n := a.received
	a.received++
	var item deliverItem
	if n < len(a.deliver) {
		item = a.deliver[n]
	}
	rc.mu.Unlock()

	status := http.StatusInternalServerError
	if router, err := rc.router(item, a.directory); err != nil {
		rc.sink.push("harness: " + err.Error())
	} else {
		onEvent := func(_ context.Context, routed lingaraembed.RoutedEvent) error {
			return rc.record(a, n, routed, item.Handler == "raise")
		}
		status = router.Handle(req.Context(), body, req.Header, onEvent)
	}
	rc.mu.Lock()
	a.answered++
	rc.mu.Unlock()
	w.WriteHeader(status)
}

func (rc *receiver) record(a *armed, n int, routed lingaraembed.RoutedEvent, raise bool) error {
	s := &seen{EventID: routed.Event.Meta().ID}
	if routed.Known {
		playerRef := routed.PlayerRef
		s.PlayerRef = &playerRef
	}
	_, s.Unknown = routed.Event.(lingara.UnknownEvent)
	rc.mu.Lock()
	if n < len(a.seen) {
		a.seen[n] = s
	}
	rc.mu.Unlock()
	if raise {
		return errors.New("the conformance handler raised, as the case asked")
	}
	return nil
}
