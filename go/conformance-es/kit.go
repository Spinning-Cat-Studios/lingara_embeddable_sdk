package main

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"reflect"
	"time"

	lingara "github.com/Spinning-Cat-Studios/lingara_api_clients/go"
	lingaraembed "github.com/Spinning-Cat-Studios/lingara_embeddable_sdk/go"
)

// job is one entry of a case's kit: list. Exactly one field is set.
type job struct {
	Mint *struct {
		PlayerRef string   `json:"player_ref"`
		Scopes    []string `json:"scopes"`
		Origin    string   `json:"origin"`
	} `json:"mint"`
	Delete *struct {
		PlayerRef string `json:"player_ref"`
	} `json:"delete"`
	Handoff *struct{} `json:"handoff"`
	Render  *struct{} `json:"render"`
}

// outcome is what the jobs left behind.
type outcome struct {
	minted  *lingara.MintedToken
	handoff *string
	err     map[string]any
	render  bool
}

// runJobs runs every job in order; a returned error is recorded and the
// next job runs.
func runJobs(ctx context.Context, server *lingaraembed.Server, jobs []job) outcome {
	var out outcome
	for _, j := range jobs {
		var err error
		switch {
		case j.Mint != nil:
			var minted *lingara.MintedToken
			minted, err = server.MintToken(ctx, j.Mint.PlayerRef, lingaraembed.MintOptions{Scopes: j.Mint.Scopes, Origin: j.Mint.Origin})
			if err == nil {
				out.minted = minted
			}
		case j.Delete != nil:
			err = server.DeletePlayer(ctx, j.Delete.PlayerRef)
		case j.Handoff != nil:
			if out.minted != nil {
				body := string(lingaraembed.TokenHandoff(out.minted))
				out.handoff = &body
			}
		default:
			out.render = true
		}
		if err != nil {
			out.err = raised(err)
		}
	}
	return out
}

// raised is {status, code}: an API error's, an OAuth error's error, or
// maintenance.
func raised(err error) map[string]any {
	var api *lingara.APIError
	var oauth *lingara.OAuthError
	var maintenance *lingara.MaintenanceError
	var transport *lingara.TransportError
	switch {
	case errors.As(err, &api):
		return map[string]any{"status": api.Status, "code": api.Code}
	case errors.As(err, &oauth):
		return map[string]any{"status": oauth.Status, "code": oauth.ErrorCode}
	case errors.As(err, &maintenance):
		return map[string]any{"status": 503, "code": "maintenance"}
	case errors.As(err, &transport):
		return map[string]any{"status": 0, "code": string(transport.Kind)}
	}
	return map[string]any{"status": 0, "code": err.Error()}
}

// wire is a MintedToken's six fields in wire form, the token read through
// its accessor.
func wire(m *lingara.MintedToken) map[string]any {
	return map[string]any{
		"token": m.Token.ExposeSecret(), "expires_at": m.ExpiresAt.UTC().Format(time.RFC3339Nano),
		"expires_in": int64(m.ExpiresIn / time.Second), "subject": m.Subject, "scopes": m.Scopes,
		"account_linked": m.AccountLinked,
	}
}

// normal is v through JSON with every null object member dropped, so two
// values compare JSON-equal.
func normal(v any) any {
	raw, err := json.Marshal(v)
	if err != nil {
		return fmt.Sprintf("unencodable: %v", err)
	}
	var decoded any
	_ = json.Unmarshal(raw, &decoded)
	return withoutNulls(decoded)
}

func withoutNulls(v any) any {
	switch t := v.(type) {
	case map[string]any:
		kept := map[string]any{}
		for k, item := range t {
			if item != nil {
				kept[k] = withoutNulls(item)
			}
		}
		return kept
	case []any:
		for i, item := range t {
			t[i] = withoutNulls(item)
		}
	}
	return v
}

// observed is what the caller saw, by expect key; an absent key was not
// observed.
func observed(out outcome, d *recording) map[string]any {
	got := map[string]any{"directory": d.Snapshot(), "directory_lookups": d.lookups.Load()}
	if out.minted != nil {
		got["minted"] = wire(out.minted)
	}
	if out.handoff != nil {
		got["handoff"] = *out.handoff
	}
	if out.err != nil {
		got["error"] = out.err
	}
	return got
}

// judge compares expect.minted, handoff, error, directory and
// directory_lookups, each a fail <key> line when it differs.
func judge(id string, expect map[string]json.RawMessage, out outcome, d *recording) []string {
	got := observed(out, d)
	var found []string
	for _, key := range []string{"minted", "handoff", "error", "directory", "directory_lookups"} {
		raw, named := expect[key]
		have, present := got[key]
		var want any
		if named {
			_ = json.Unmarshal(raw, &want)
			want = withoutNulls(want)
		}
		if differs(key, named, want, present, have) {
			found = append(found, fmt.Sprintf("%s: fail %s: got %s, expected %s", id, key, show(have, present), show(want, named)))
		}
	}
	return found
}

// differs: a key the case names must match JSON-equal; an error the case
// did not expect is a mismatch too.
func differs(key string, named bool, want any, present bool, have any) bool {
	if !named {
		return key == "error" && present
	}
	return !present || !reflect.DeepEqual(normal(have), want)
}

func show(v any, present bool) string {
	if !present {
		return "nothing"
	}
	raw, _ := json.Marshal(v)
	return string(raw)
}
