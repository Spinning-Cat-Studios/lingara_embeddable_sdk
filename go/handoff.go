package lingaraembed

import (
	"fmt"
	"time"

	lingara "github.com/Spinning-Cat-Studios/lingara_api_clients/go"
)

// TokenHandoff is the JSON body your token route returns to the player (ADR
// 1.10.26af D6, ES6): exactly {"token":"lgr_et_…","expires_at":"…"}, the
// shape every player kit and the widget read. The library holds expires_at
// as a time.Time, so it is written back as RFC 3339 in UTC with a Z, with
// fractional seconds only when non-zero. expires_in and subject are not
// forwarded. This is the one place the kit exposes the token, on purpose.
func TokenHandoff(m *lingara.MintedToken) []byte {
	return handoffBody(m.Token.ExposeSecret(), expiresAt(m.ExpiresAt))
}

// expiresAt is ES6's instant: UTC, Z, trailing fractional zeros trimmed.
// RFC3339Nano already trims them, and drops the dot when none remain.
func expiresAt(t time.Time) string {
	return t.UTC().Format(time.RFC3339Nano)
}

// handoffBody is the pure body over two strings, held to token-handoff.json:
// compact, these two keys in this order, nothing reformatted.
func handoffBody(token, expiresAt string) []byte {
	return []byte(`{"token":` + jsonString(token) + `,"expires_at":` + jsonString(expiresAt) + `}`)
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
