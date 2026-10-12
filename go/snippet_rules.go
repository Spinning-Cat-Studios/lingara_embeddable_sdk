package lingaraembed

import (
	"regexp"
	"strings"
	"unicode/utf8"
)

// ES4's option rules (ADR 1.10.26af D7), one function per row of the
// contract's table. Every pattern is ASCII-only, and Go's $ matches only at
// the end of the text, so a trailing newline is refused.

var (
	planIDPattern  = regexp.MustCompile(`^[0-9A-Fa-f]{8}(-[0-9A-Fa-f]{4}){3}-[0-9A-Fa-f]{12}$`)
	surfacePattern = regexp.MustCompile(`^[a-z][a-z0-9-]{0,31}$`)
	localePattern  = regexp.MustCompile(`^[A-Za-z]{2,3}(-[A-Za-z0-9]{2,8}){0,3}$`)
	originPattern  = regexp.MustCompile(`^(https://[A-Za-z0-9-]+(\.[A-Za-z0-9-]+)*(:[0-9]{1,5})?|http://(localhost|127\.0\.0\.1)(:[0-9]{1,5})?)/?$`)
	idPattern      = regexp.MustCompile(`^[A-Za-z][A-Za-z0-9_-]{0,63}$`)
	noncePattern   = regexp.MustCompile(`^[A-Za-z0-9+/=_-]{8,128}$`)
)

// isControl is Unicode category Cc.
func isControl(r rune) bool {
	return r <= 0x1f || (r >= 0x7f && r <= 0x9f)
}

// isSpace is the contract's whitespace set (Unicode White_Space beyond Cc).
func isSpace(r rune) bool {
	switch r {
	case ' ', 0x85, 0xa0, 0x1680, 0x2028, 0x2029, 0x202f, 0x205f, 0x3000:
		return true
	}
	return r >= 0x2000 && r <= 0x200a
}

func isTheme(s string) bool {
	return s == "light" || s == "dark" || s == "auto"
}

// isTitle is 1–80 scalar values, none of them a control character.
func isTitle(s string) bool {
	n := utf8.RuneCountInString(s)
	return n >= 1 && n <= 80 && strings.IndexFunc(s, isControl) < 0
}

// isURLText is at most 2048 bytes, with no control character or whitespace.
func isURLText(s string) bool {
	return len(s) <= 2048 && strings.IndexFunc(s, func(r rune) bool { return isControl(r) || isSpace(r) }) < 0
}

// isTokenEndpoint is a path that starts / but not //, or an https:// URL.
func isTokenEndpoint(s string) bool {
	shape := (strings.HasPrefix(s, "/") && !strings.HasPrefix(s, "//")) || strings.HasPrefix(s, "https://")
	return shape && isURLText(s)
}

func isElementSrc(s string) bool {
	return strings.HasPrefix(s, "https://") && isURLText(s)
}

// rule is one row: an option's contract name, its value (nil when absent)
// and its check.
type rule struct {
	field string
	value *string
	ok    func(string) bool
}

// refused is the first rule a present value breaks, in the contract's
// order, or "".
func refused(o SnippetOptions, surface, id string) string {
	rules := []rule{
		{"planId", &o.PlanID, planIDPattern.MatchString},
		{"surface", &surface, surfacePattern.MatchString},
		{"theme", o.Theme, isTheme},
		{"locale", o.Locale, localePattern.MatchString},
		{"lingaraOrigin", o.LingaraOrigin, originPattern.MatchString},
		{"title", o.Title, isTitle},
		{"id", &id, idPattern.MatchString},
		{"tokenEndpoint", &o.TokenEndpoint, isTokenEndpoint},
		{"nonce", o.Nonce, noncePattern.MatchString},
		{"elementSrc", o.ElementSrc, isElementSrc},
	}
	for _, r := range rules {
		if r.value != nil && !r.ok(*r.value) {
			return r.field
		}
	}
	return ""
}
