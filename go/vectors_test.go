package lingaraembed

import (
	"encoding/json"
	"errors"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// readVectors decodes conformance/vectors/<name>, read from the repository
// root as every server kit reads it.
func readVectors(t *testing.T, name string, into any) {
	t.Helper()
	raw, err := os.ReadFile(filepath.Join("..", "conformance", "vectors", name))
	if err != nil {
		t.Fatal(err)
	}
	if err := json.Unmarshal(raw, into); err != nil {
		t.Fatalf("%s: %v", name, err)
	}
}

type snippetVector struct {
	Name    string             `json:"name"`
	Options map[string]*string `json:"options"`
	Expect  struct {
		HTML    *string `json:"html"`
		Refused *string `json:"refused"`
	} `json:"expect"`
}

// snippetOptions maps the vectors' camelCase options onto SnippetOptions.
func snippetOptions(o map[string]*string) SnippetOptions {
	text := func(key string) string {
		if v := o[key]; v != nil {
			return *v
		}
		return ""
	}
	return SnippetOptions{
		PlanID: text("planId"), TokenEndpoint: text("tokenEndpoint"),
		Surface: o["surface"], Theme: o["theme"], Locale: o["locale"], LingaraOrigin: o["lingaraOrigin"],
		Title: o["title"], ID: o["id"], Nonce: o["nonce"], ElementSrc: o["elementSrc"],
	}
}

func checkSnippet(t *testing.T, v snippetVector) {
	t.Helper()
	got, err := WidgetSnippet(snippetOptions(v.Options))
	var refusal *SnippetError
	switch {
	case v.Expect.HTML != nil:
		want := strings.ReplaceAll(*v.Expect.HTML, "{{version}}", Version)
		if err != nil || got != want {
			t.Errorf("%s: got %q, %v\nwant %q", v.Name, got, err, want)
		}
	case v.Expect.Refused != nil:
		if !errors.As(err, &refusal) || refusal.Field != *v.Expect.Refused {
			t.Errorf("%s: got %q, %v; want refused %s", v.Name, got, err, *v.Expect.Refused)
		}
	default:
		t.Errorf("%s: expect is html or refused", v.Name)
	}
}

// TestEverySnippetAndHandoffVector: 1.10.26af AC6. Every snippet.json vector
// through WidgetSnippet, and every token-handoff.json vector through the
// internal handoffBody, gives its expected answer.
func TestEverySnippetAndHandoffVector(t *testing.T) {
	var snippets struct {
		Cases []snippetVector `json:"cases"`
	}
	readVectors(t, "snippet.json", &snippets)
	if len(snippets.Cases) < 40 {
		t.Fatalf("only %d snippet vectors", len(snippets.Cases))
	}
	for _, v := range snippets.Cases {
		checkSnippet(t, v)
	}
	var handoffs []struct {
		Name  string `json:"name"`
		Input struct {
			Token     string `json:"token"`
			ExpiresAt string `json:"expires_at"`
		} `json:"input"`
		Body string `json:"body"`
	}
	readVectors(t, "token-handoff.json", &handoffs)
	if len(handoffs) == 0 {
		t.Fatal("no handoff vectors")
	}
	for _, v := range handoffs {
		if got := string(handoffBody(v.Input.Token, v.Input.ExpiresAt)); got != v.Body {
			t.Errorf("%s: got %s, want %s", v.Name, got, v.Body)
		}
	}
}
