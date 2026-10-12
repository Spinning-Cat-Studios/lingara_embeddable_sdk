// Package lingaraembed is the Go server kit of Lingara's embed SDK (ADR
// 1.10.26af): the four jobs a game's or a site's server does for its
// players. It mints a player's token (Server.MintToken) and hands it over
// (TokenHandoff), renders the widget snippet (WidgetSnippet), routes a
// verified webhook to the player it concerns (PlayerRouter), and deletes a
// player (Server.DeletePlayer).
//
// The kit is a thin layer over the lingara library: the calls, the token
// cache, the retries, the errors and the webhook verifier are all the
// library's. The one thing it asks of you is a PlayerDirectory over your own
// store.
package lingaraembed

import (
	"context"
	"fmt"
	"log/slog"

	lingara "github.com/Spinning-Cat-Studios/lingara_api_clients/go"
)

// MintOptions is what a mint may send beside the player. Each is sent only
// when given: Scopes when non-nil, Origin when non-empty.
type MintOptions struct {
	// Scopes narrows the token to these handable scopes. Nil asks for every
	// handable scope your client holds.
	Scopes []string
	// Origin is your page's origin, one of your client's registered embed
	// origins. A game's server mints without one. The kit never reads it
	// from a request: it is yours to pass.
	Origin string
}

// ServerOption configures a Server.
type ServerOption func(*Server)

// WithDirectory records each mint's subject in d, and forgets a deleted
// player there.
func WithDirectory(d PlayerDirectory) ServerOption {
	return func(s *Server) { s.directory = d }
}

// Server holds your lingara client and, optionally, your PlayerDirectory.
// It is safe for concurrent use, and it caches no player token: every
// MintToken is one request, so a token DeletePlayer revoked on another
// instance is never handed out again.
type Server struct {
	client    *lingara.Client
	directory PlayerDirectory
}

// NewServer wraps a client built with your client credentials. Its token
// cache, retries and deprecation hook are the library's.
func NewServer(c *lingara.Client, opts ...ServerOption) *Server {
	s := &Server{client: c}
	for _, opt := range opts {
		opt(s)
	}
	return s
}

// MintToken mints a token for one player: one POST /v1/embed/tokens, then
// Remember(playerRef, subject) when a directory is configured. A failed call
// returns the library's error unchanged and remembers nothing; a failed
// Remember is a *DirectoryError.
func (s *Server) MintToken(ctx context.Context, playerRef string, o MintOptions) (*lingara.MintedToken, error) {
	req := lingara.EmbedTokenRequest{PlayerRef: playerRef}
	if o.Scopes != nil {
		scopes := make([]lingara.EmbedTokenRequestScopes, len(o.Scopes))
		for i, scope := range o.Scopes {
			scopes[i] = lingara.EmbedTokenRequestScopes(scope)
		}
		req.Scopes = &scopes
	}
	if o.Origin != "" {
		origin := o.Origin
		req.Origin = &origin
	}
	res, err := s.client.CreateEmbedToken(ctx, req)
	if err != nil {
		return nil, err
	}
	minted := res.Value
	if s.directory != nil {
		if err := s.directory.Remember(ctx, playerRef, minted.Subject); err != nil {
			return nil, &DirectoryError{Err: err}
		}
	}
	return &minted, nil
}

// DeletePlayer deletes a player and revokes their tokens, then forgets them
// when a directory is configured. An unknown player is a success. On a
// failure the library's error is returned unchanged and the row is kept,
// since the player still exists at Lingara. Call it from your own
// account-deletion path.
func (s *Server) DeletePlayer(ctx context.Context, playerRef string) error {
	if _, err := s.client.DeleteEmbedPlayer(ctx, playerRef); err != nil {
		return err
	}
	if s.directory != nil {
		if err := s.directory.Forget(ctx, playerRef); err != nil {
			return &DirectoryError{Err: err}
		}
	}
	return nil
}

// Format renders the client through its own redacting Format.
func (s *Server) Format(f fmt.State, verb rune) {
	if verb == 'v' && (f.Flag('+') || f.Flag('#')) {
		_, _ = fmt.Fprintf(f, "&lingaraembed.Server{Client:%+v, Directory:%t}", s.client, s.directory != nil)
		return
	}
	_, _ = fmt.Fprintf(f, "lingaraembed.Server(directory=%t)", s.directory != nil)
}

// LogValue is whether a directory is configured, and the client's own
// redacted group.
func (s *Server) LogValue() slog.Value {
	return slog.GroupValue(slog.Any("client", s.client), slog.Bool("directory", s.directory != nil))
}
