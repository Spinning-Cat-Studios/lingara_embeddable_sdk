package snippets

import (
	"html/template"

	lingaraembed "github.com/Spinning-Cat-Studios/lingara_embeddable_sdk/go"
)

// WidgetFor is the widget example.
func WidgetFor(planID, cspNonce string) (template.HTML, error) {
	// lingara:begin widgetSnippet
	// The plan your server made for this player, and the token route above.
	// Pass the request's CSP nonce when your script-src allows no inline
	// script; the token itself never appears in the markup.
	theme := "auto"
	html, err := lingaraembed.WidgetSnippet(lingaraembed.SnippetOptions{
		PlanID:        planID,
		TokenEndpoint: "/lingara/token",
		Theme:         &theme,
		Nonce:         &cspNonce,
	})
	if err != nil {
		return "", err // a *lingaraembed.SnippetError names the refused option
	}
	return template.HTML(html), nil // already escaped by the kit
	// lingara:end
}
