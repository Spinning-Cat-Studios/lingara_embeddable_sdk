package lingaraembed

import (
	"os"
	"strings"
	"testing"
)

// requires lists go.mod's required module paths, from single-line `require`
// directives and require blocks alike.
func requires(gomod string) []string {
	var paths []string
	inBlock := false
	for _, line := range strings.Split(gomod, "\n") {
		line = strings.TrimSpace(strings.SplitN(line, "//", 2)[0])
		switch {
		case line == "require (":
			inBlock = true
		case inBlock && line == ")":
			inBlock = false
		case inBlock && line != "":
			paths = append(paths, strings.Fields(line)[0])
		case strings.HasPrefix(line, "require "):
			paths = append(paths, strings.Fields(line)[1])
		}
	}
	return paths
}

// TestTheOnlyRequireIsTheLibrary: 1.10.26af AC22. go.mod has exactly one
// require, the lingara library module (the dependency rule, D1); it asks
// for no newer Go than the floor, go 1.23, and replaces nothing, so the kit
// depends on the released library.
func TestTheOnlyRequireIsTheLibrary(t *testing.T) {
	raw, err := os.ReadFile("go.mod")
	if err != nil {
		t.Fatal(err)
	}
	gomod := string(raw)
	got := requires(gomod)
	if len(got) != 1 || got[0] != "github.com/Spinning-Cat-Studios/lingara_api_clients/go" {
		t.Errorf("go.mod requires %v", got)
	}
	if !strings.Contains(gomod, "\ngo 1.23\n") || strings.Contains(gomod, "\ntoolchain ") {
		t.Error("go.mod's go directive is not the floor, go 1.23")
	}
	if strings.Contains(gomod, "\nreplace ") {
		t.Error("go.mod replaces a module: a kit depends on the released library")
	}
}
