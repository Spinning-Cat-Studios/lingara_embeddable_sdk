// Package snippets holds the Go server kit's examples the docs show. Each
// region builds in make test-go, so an example cannot drift from the kit.
package snippets

import (
	"errors"
	"net/http"
	"os"

	lingara "github.com/Spinning-Cat-Studios/lingara_api_clients/go"
	lingaraembed "github.com/Spinning-Cat-Studios/lingara_embeddable_sdk/go"
)

// NewEmbedServer is built once, at startup, from your client credentials.
func NewEmbedServer(players lingaraembed.PlayerDirectory) (*lingaraembed.Server, error) {
	client, err := lingara.New(lingara.WithClientCredentials(os.Getenv("LINGARA_CLIENT_ID"), os.Getenv("LINGARA_CLIENT_SECRET")))
	if err != nil {
		return nil, err
	}
	return lingaraembed.NewServer(client, lingaraembed.WithDirectory(players)), nil
}

// currentPlayer is your own session lookup.
func currentPlayer(*http.Request) (string, bool) { return "", false }

// TokenRoute is the mint example.
func TokenRoute(embed *lingaraembed.Server) http.HandlerFunc {
	// lingara:begin mint
	// POST /lingara/token: your own route, behind your own session, and
	// rate-limited per session. The widget calls it with same-origin cookies.
	return func(w http.ResponseWriter, r *http.Request) {
		playerRef, ok := currentPlayer(r) // your user, never a value from the request body
		if !ok {
			http.Error(w, "sign in first", http.StatusUnauthorized)
			return
		}
		minted, err := embed.MintToken(r.Context(), playerRef, lingaraembed.MintOptions{
			Origin: "https://game.example", // this page's origin, one of your client's embed origins
		})
		if err != nil {
			http.Error(w, "no token", http.StatusBadGateway)
			return
		}
		w.Header().Set("Content-Type", "application/json")
		w.Header().Set("Cache-Control", "no-store")
		_, _ = w.Write(lingaraembed.TokenHandoff(minted)) // {"token":"lgr_et_…","expires_at":"…"}
	}
	// lingara:end
}

// DeleteAccount is the delete example.
func DeleteAccount(embed *lingaraembed.Server, r *http.Request, playerRef string) error {
	// lingara:begin deletePlayer
	// From your own account-deletion path: revokes every token the player
	// holds and forgets their subjects. An unknown player is a success.
	if err := embed.DeletePlayer(r.Context(), playerRef); err != nil {
		var apiErr *lingara.APIError
		if errors.As(err, &apiErr) {
			return apiErr // the library's own error: Status, Code, RetryAfter
		}
		return err
	}
	// lingara:end
	return nil
}
