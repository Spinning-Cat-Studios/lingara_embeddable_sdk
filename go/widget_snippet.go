package lingaraembed

import (
	"fmt"
	"strings"
)

// DefaultElementSrc is the script-tag build of this release's element.
const DefaultElementSrc = "https://cdn.jsdelivr.net/npm/@lingara/embed@" + Version + "/dist/lingara-embed.min.js"

// SnippetOptions is what WidgetSnippet renders (ADR 1.10.26af D7). PlanID
// and TokenEndpoint are required; every pointer is an option written only
// when non-nil.
type SnippetOptions struct {
	// PlanID is the lesson plan your server made for this player, a UUID.
	PlanID string
	// TokenEndpoint is your token route: a path (/lingara/token) or an
	// https:// URL. The element POSTs to it with the page's same-origin
	// credentials and reads TokenHandoff's body. It never holds a token.
	TokenEndpoint string
	// Surface defaults to practice.
	Surface *string
	// Theme is light, dark or auto.
	Theme         *string
	Locale        *string
	LingaraOrigin *string
	Title         *string
	// ID defaults to lingara-embed; two widgets on one page need two ids.
	ID *string
	// Nonce is your page's CSP nonce, written on both scripts. Without it a
	// page whose script-src allows no inline script blocks the third line.
	Nonce *string
	// ElementSrc defaults to DefaultElementSrc; an override is https://.
	ElementSrc *string
}

// SnippetError names the first option WidgetSnippet refused, by its
// contract name: planId, surface, theme, locale, lingaraOrigin, title, id,
// tokenEndpoint, nonce or elementSrc. It never quotes the value.
type SnippetError struct {
	Field string
}

func (e *SnippetError) Error() string {
	return "lingaraembed: the widget snippet's " + e.Field + " is not valid"
}

// WidgetSnippet returns the three lines to paste into your page, with no
// trailing newline: the <lingara-embed> element, its module script, and an
// inline module that sets the element's tokenProvider to your endpoint. A
// refused option is a *SnippetError.
func WidgetSnippet(o SnippetOptions) (string, error) {
	surface := valueOr(o.Surface, "practice")
	id := valueOr(o.ID, "lingara-embed")
	if field := refused(o, surface, id); field != "" {
		return "", &SnippetError{Field: field}
	}
	var b strings.Builder
	b.WriteString(`<lingara-embed id="` + attr(id) + `" surface="` + attr(surface) + `" plan-id="` + attr(o.PlanID) + `"`)
	for _, a := range []struct {
		name  string
		value *string
	}{{"theme", o.Theme}, {"locale", o.Locale}, {"lingara-origin", o.LingaraOrigin}, {"title", o.Title}} {
		if a.value != nil {
			b.WriteString(" " + a.name + `="` + attr(*a.value) + `"`)
		}
	}
	nonce := ""
	if o.Nonce != nil {
		nonce = ` nonce="` + attr(*o.Nonce) + `"`
	}
	b.WriteString("></lingara-embed>\n")
	b.WriteString(`<script type="module"` + nonce + ` src="` + attr(valueOr(o.ElementSrc, DefaultElementSrc)) + `"></script>` + "\n")
	b.WriteString(`<script type="module"` + nonce + `>document.getElementById(` + scriptString(id) +
		`).tokenProvider = async () => (await fetch(` + scriptString(o.TokenEndpoint) +
		`, { method: "POST", credentials: "same-origin" })).json();</script>`)
	return b.String(), nil
}

func valueOr(v *string, fallback string) string {
	if v == nil {
		return fallback
	}
	return *v
}

// attrEscaper escapes an attribute value's five characters and nothing
// else. html.EscapeString writes &#34; for '"', which the vectors refuse.
var attrEscaper = strings.NewReplacer("&", "&amp;", "<", "&lt;", ">", "&gt;", `"`, "&quot;", "'", "&#39;")

func attr(s string) string { return attrEscaper.Replace(s) }

// scriptString is a JSON string literal that can never close the script or
// open a comment: '"' and '\' escaped as JSON requires, and < > & U+2028
// U+2029 as \u escapes. Nothing else changes.
func scriptString(s string) string {
	var b strings.Builder
	b.WriteByte('"')
	for _, r := range s {
		switch r {
		case '"', '\\':
			b.WriteByte('\\')
			b.WriteRune(r)
		case '<', '>', '&', '\u2028', '\u2029':
			fmt.Fprintf(&b, `\u%04x`, r)
		default:
			b.WriteRune(r)
		}
	}
	b.WriteByte('"')
	return b.String()
}
