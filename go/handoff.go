package lingaraembed

import (
	"fmt"
	"time"

	lingara "github.com/Spinning-Cat-Studios/lingara_api_clients/go"
)

// TokenHandoff is the JSON body your token route returns to the player (ADR
// 1.10.26af D6, ES6): {"token":"lgr_et_…","expires_at":"…"}, the shape every
// player kit and the widget read, plus a third key update_available when the
// mint answered one (ADR 31.10.26f D1): {"latest","deprecated","message"} in
// that order, null kept. Without it the body is exactly the two keys. The
// library holds expires_at as a time.Time, so it is written back as RFC 3339
// in UTC with a Z, with fractional seconds only when non-zero. expires_in and
// subject are not forwarded. This is the one place the kit exposes the token,
// on purpose.
func TokenHandoff(m *lingara.MintedToken) []byte {
	return handoffBody(m.Token.ExposeSecret(), expiresAt(m.ExpiresAt), m.UpdateAvailable)
}

// expiresAt is ES6's instant: UTC, Z, trailing fractional zeros trimmed.
// RFC3339Nano already trims them, and drops the dot when none remain.
func expiresAt(t time.Time) string {
	return t.UTC().Format(time.RFC3339Nano)
}

// handoffBody is the pure body, held to token-handoff.json: compact, token
// then expires_at, then update_available only when non-nil, nothing
// reformatted.
func handoffBody(token, expiresAt string, update *lingara.UpdateAvailable) []byte {
	body := `{"token":` + jsonString(token) + `,"expires_at":` + jsonString(expiresAt)
	if update != nil {
		body += `,"update_available":` + updateJSON(update)
	}
	return []byte(body + `}`)
}

// updateJSON writes the notice's three keys in V1's order, every one always
// present. It is hand-written rather than encoding/json, which would escape
// < > & in the message, against ES6.
func updateJSON(u *lingara.UpdateAvailable) string {
	deprecated := "false"
	if u.Deprecated {
		deprecated = "true"
	}
	return `{"latest":` + nullableJSON(u.Latest) + `,"deprecated":` + deprecated +
		`,"message":` + nullableJSON(u.Message) + `}`
}

// nullableJSON is a string literal, or null for nil.
func nullableJSON(s *string) string {
	if s == nil {
		return "null"
	}
	return jsonString(*s)
}

// jsonString is a JSON string literal escaping only what JSON requires: '"',
// '\' and the C0 controls. encoding/json would also escape < > & U+2028
// U+2029, which ES6 forbids.
func jsonString(s string) string {
	b := make([]byte, 0, len(s)+2)
	b = append(b, '"')
	for i := 0; i < len(s); i++ {
		switch c := s[i]; {
		case c == '"' || c == '\\':
			b = append(b, '\\', c)
		case c == '\n':
			b = append(b, '\\', 'n')
		case c == '\r':
			b = append(b, '\\', 'r')
		case c == '\t':
			b = append(b, '\\', 't')
		case c < 0x20:
			b = fmt.Appendf(b, `\u%04x`, c)
		default:
			b = append(b, c)
		}
	}
	return string(append(b, '"'))
}
