// Command conformance-es is the ES harness on the Go server kit (ADR
// 1.10.26af D13): every ES case through package lingaraembed's public API
// and nothing else, on net/http. It is conformance-only, a module of its
// own, so neither it nor its replace directive reaches the kit's module.
//
// `conformance-server run --tier es --lang go -- <this>` starts it with the
// server's URL, the results path, the receiver's two webhook secrets and the
// visibly fake client credentials. It binds its webhook receiver to port 0,
// prints `listening <port>`, and then for each case: fetches it, builds a
// fresh client, server and directory, arms it, runs the kit: jobs, waits for
// every delivery, judges what only the caller sees, finishes, and writes one
// result line. There is no skip.
package main

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"net"
	"net/http"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"time"

	lingara "github.com/Spinning-Cat-Studios/lingara_api_clients/go"
	lingaraembed "github.com/Spinning-Cat-Studios/lingara_embeddable_sdk/go"
)

// lang is the language this harness reports: make/go.mk's ES_LANGS entry.
const lang = "go"

var secretPrefixes = []string{"lgr_et_", "lgr_cs_", "lgr_whsec_"}

// esCase is the part of a case the harness reads.
type esCase struct {
	Directory struct {
		Seed map[string]string `json:"seed"`
	} `json:"directory"`
	Kit     []job                      `json:"kit"`
	Deliver []deliverItem              `json:"deliver"`
	Expect  map[string]json.RawMessage `json:"expect"`
}

func main() {
	if err := run(); err != nil {
		fmt.Fprintln(os.Stderr, "✗ conformance-es:", err)
		os.Exit(1)
	}
}

func run() error {
	logs := &sink{}
	captureLogs(logs)
	named := strings.Split(os.Getenv("LINGARA_EMBED_WEBHOOK_SECRETS"), ",")
	rc := &receiver{secrets: map[string]string{}, sink: logs}
	for i, name := range []string{"primary", "secondary"} {
		if i < len(named) {
			rc.secrets[name] = named[i]
		}
	}
	listener, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		return err
	}
	// The conformance server waits for this as the first line of stdout.
	fmt.Printf("listening %d\n", listener.Addr().(*net.TCPAddr).Port)
	_ = os.Stdout.Sync()
	go func() { _ = http.Serve(listener, rc) }()

	out, err := os.Create(os.Getenv("LINGARA_EMBED_RESULTS"))
	if err != nil {
		return err
	}
	defer func() { _ = out.Close() }()
	cases := os.Getenv("LINGARA_EMBED_CASES")
	if cases == "" {
		cases = filepath.Join("conformance", "es", "cases")
	}
	for _, id := range caseIDs(cases) {
		line, _ := json.Marshal(runCase(rc, id))
		if _, err := fmt.Fprintf(out, "%s\n", line); err != nil {
			return err
		}
	}
	return nil
}

// caseIDs maps <group>/<slug>.yaml to <group>.<slug>, sorted.
func caseIDs(dir string) []string {
	files, _ := filepath.Glob(filepath.Join(dir, "*", "*.yaml"))
	ids := make([]string, 0, len(files))
	for _, f := range files {
		ids = append(ids, filepath.Base(filepath.Dir(f))+"."+strings.TrimSuffix(filepath.Base(f), ".yaml"))
	}
	sort.Strings(ids)
	return ids
}

func result(id string, mismatches []string) map[string]any {
	verdict := "pass"
	if len(mismatches) > 0 {
		verdict = "fail"
	}
	if mismatches == nil {
		mismatches = []string{}
	}
	return map[string]any{"case": id, "lang": lang, "result": verdict, "mismatches": mismatches}
}

func runCase(rc *receiver, id string) map[string]any {
	base := os.Getenv("LINGARA_EMBED_SERVER_URL")
	control := func(verb string) string { return base + "/__conformance/" + verb + "/" + id }
	var c esCase
	if err := getJSON(control("cases"), &c); err != nil {
		return result(id, []string{"cannot read the case: " + err.Error()})
	}
	directory := seeded(c.Directory.Seed)
	rc.mu.Lock()
	rc.armed = &armed{deliver: c.Deliver, directory: directory, seen: make([]*seen, len(c.Deliver))}
	rc.mu.Unlock()
	rc.sink.take()
	client, err := lingara.New(
		lingara.WithClientCredentials(os.Getenv("LINGARA_EMBED_CLIENT_ID"), os.Getenv("LINGARA_EMBED_CLIENT_SECRET")),
		lingara.WithBaseURL(base), lingara.WithTokenURL(base+"/oauth/token"))
	if err != nil {
		return result(id, []string{"cannot build a client: " + err.Error()})
	}
	server := lingaraembed.NewServer(client, lingaraembed.WithDirectory(directory))

	var mismatches []string
	if err := post(control("arm"), nil, nil); err != nil {
		mismatches = append(mismatches, "cannot arm: "+err.Error())
	}
	out := runJobs(context.Background(), server, c.Kit)
	saw := waitForDeliveries(rc, len(c.Deliver))
	mismatches = append(mismatches, judge(id, c.Expect, out, directory)...)
	mismatches = append(mismatches, handlerMismatches(id, c.Deliver, saw)...)
	if out.render {
		mismatches = append(mismatches, renderMismatches(id, rc, server, directory, out)...)
	}
	handlers := make([]*string, len(saw))
	for i, s := range saw {
		if s != nil {
			handlers[i] = s.PlayerRef
		}
	}
	var finished struct {
		Mismatches []string `json:"mismatches"`
	}
	if err := post(control("finish"), map[string]any{"handlers": handlers}, &finished); err != nil {
		mismatches = append(mismatches, "cannot finish: "+err.Error())
	}
	rc.mu.Lock()
	rc.armed = nil
	rc.mu.Unlock()
	return result(id, append(mismatches, finished.Mismatches...))
}

func getJSON(url string, into any) error {
	res, err := http.Get(url)
	if err != nil {
		return err
	}
	defer func() { _ = res.Body.Close() }()
	if res.StatusCode >= 300 {
		return fmt.Errorf("HTTP %d", res.StatusCode)
	}
	return json.NewDecoder(res.Body).Decode(into)
}

func post(url string, body, into any) error {
	raw, _ := json.Marshal(body)
	if body == nil {
		raw = nil
	}
	res, err := http.Post(url, "application/json", bytes.NewReader(raw))
	if err != nil {
		return err
	}
	defer func() { _ = res.Body.Close() }()
	if res.StatusCode >= 300 {
		return fmt.Errorf("HTTP %d", res.StatusCode)
	}
	if into == nil {
		return nil
	}
	return json.NewDecoder(res.Body).Decode(into)
}

// waitForDeliveries waits until every delivery has been answered, or 15 s,
// and returns what each saw.
func waitForDeliveries(rc *receiver, count int) []*seen {
	deadline := time.Now().Add(15 * time.Second)
	for {
		rc.mu.Lock()
		done := rc.armed == nil || rc.armed.answered >= count
		if done || time.Now().After(deadline) {
			var saw []*seen
			if rc.armed != nil {
				saw = append(saw, rc.armed.seen...)
			}
			rc.mu.Unlock()
			return saw
		}
		rc.mu.Unlock()
		time.Sleep(20 * time.Millisecond)
	}
}

// handlerMismatches compares each delivery's expect.handler with what its
// onEvent saw.
func handlerMismatches(id string, items []deliverItem, saw []*seen) []string {
	var found []string
	for n, item := range items {
		var got *seen
		if n < len(saw) {
			got = saw[n]
		}
		if !handlerMatches(item.Expect.Handler, got) {
			found = append(found, fmt.Sprintf("%s: fail handler: deliver[%d] saw %s, expected %s", id, n, show(got, got != nil), show(item.Expect.Handler, true)))
		}
	}
	return found
}

func handlerMatches(want any, got *seen) bool {
	if want == "not_called" {
		return got == nil
	}
	fields, ok := want.(map[string]any)
	if !ok || got == nil {
		return false
	}
	var playerRef any
	if got.PlayerRef != nil {
		playerRef = *got.PlayerRef
	}
	return fieldMatches(fields, "player_ref", playerRef) && fieldMatches(fields, "event_id", got.EventID) &&
		fieldMatches(fields, "unknown", got.Unknown)
}

// fieldMatches is true when expect.handler does not name key, or names
// exactly have (a null player_ref expects no player).
func fieldMatches(fields map[string]any, key string, have any) bool {
	want, named := fields[key]
	return !named || want == have
}

// renderMismatches is ES5: no rendering of a kit object, and no captured
// log line, carries a secret prefix.
func renderMismatches(id string, rc *receiver, server *lingaraembed.Server, d *recording, out outcome) []string {
	router, err := rc.router(deliverItem{Router: &struct {
		Secrets []string `json:"secrets"`
	}{Secrets: []string{"primary", "secondary"}}}, d)
	if err != nil {
		return []string{id + ": fail render: cannot build a router: " + err.Error()}
	}
	var renderings []string
	for _, verb := range []string{"%v", "%+v", "%#v", "%s"} {
		renderings = append(renderings, fmt.Sprintf(verb, server), fmt.Sprintf(verb, router), fmt.Sprintf(verb, d.MemoryDirectory))
		if out.minted != nil {
			renderings = append(renderings, fmt.Sprintf(verb, out.minted), fmt.Sprintf(verb, *out.minted), fmt.Sprintf(verb, out.minted.Token))
		}
	}
	if out.minted != nil {
		raw, _ := json.Marshal(out.minted)
		renderings = append(renderings, string(raw))
	}
	renderings = append(renderings, rc.sink.take()...)
	leaks := 0
	for _, r := range renderings {
		for _, prefix := range secretPrefixes {
			if strings.Contains(r, prefix) {
				leaks++
				break
			}
		}
	}
	if leaks == 0 {
		return nil
	}
	return []string{fmt.Sprintf("%s: fail render: %d rendering(s) carry a secret prefix", id, leaks)}
}
