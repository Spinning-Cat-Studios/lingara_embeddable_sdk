package snippets

import (
	"context"
	"database/sql"
	"errors"
	"io"
	"log/slog"
	"net/http"
	"os"

	lingara "github.com/Spinning-Cat-Studios/lingara_api_clients/go"
	lingaraembed "github.com/Spinning-Cat-Studios/lingara_embeddable_sdk/go"
)

// lingara:begin webhook
// lingara_players(player_ref TEXT NOT NULL, subject TEXT NOT NULL UNIQUE),
// with a plain index on player_ref: one player may own several subjects.
type sqlPlayers struct{ db *sql.DB }

func (p sqlPlayers) Remember(ctx context.Context, playerRef, subject string) error {
	_, err := p.db.ExecContext(ctx, `INSERT INTO lingara_players (player_ref, subject) VALUES ($1, $2)
		ON CONFLICT (subject) DO UPDATE SET player_ref = excluded.player_ref`, playerRef, subject)
	return err
}

func (p sqlPlayers) PlayerRefFor(ctx context.Context, subject string) (string, bool, error) {
	var playerRef string
	err := p.db.QueryRowContext(ctx, `SELECT player_ref FROM lingara_players WHERE subject = $1`, subject).Scan(&playerRef)
	if errors.Is(err, sql.ErrNoRows) {
		return "", false, nil
	}
	return playerRef, err == nil, err
}

func (p sqlPlayers) Forget(ctx context.Context, playerRef string) error {
	_, err := p.db.ExecContext(ctx, `DELETE FROM lingara_players WHERE player_ref = $1`, playerRef)
	return err
}

// WebhookRoute is the webhook example.
func WebhookRoute(db *sql.DB) (http.HandlerFunc, error) {
	router, err := lingaraembed.NewPlayerRouter(sqlPlayers{db}, os.Getenv("LINGARA_WEBHOOK_SECRET"))
	if err != nil {
		return nil, err // a malformed lgr_whsec_… fails here, at startup
	}
	return func(w http.ResponseWriter, r *http.Request) {
		body, err := io.ReadAll(r.Body) // the raw body: verify before parsing
		if err != nil {
			w.WriteHeader(http.StatusBadRequest)
			return
		}
		w.WriteHeader(router.Handle(r.Context(), body, r.Header, onEvent))
	}, nil
}

func onEvent(ctx context.Context, routed lingaraembed.RoutedEvent) error {
	// Delivery is at least once: skip an event.Meta().ID you have seen.
	if e, ok := routed.Event.(lingara.PracticeCompleted); ok && routed.Known {
		slog.InfoContext(ctx, "practice completed", "player", routed.PlayerRef, "plan", e.Data.PlanID)
	}
	return nil // a returned error answers 500, and Lingara redelivers
}

// lingara:end
