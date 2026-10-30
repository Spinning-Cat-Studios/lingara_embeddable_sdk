extends SceneTree
## The EK harness through LingaraEmbed (ADR 1.10.26ab D10, AC4, AC5).
##
## `conformance-server run --tier ek --lang godot -- <this>` starts it with
## LINGARA_EMBED_SERVER_URL and LINGARA_EMBED_RESULTS. For each case under
## conformance/ek/cases/ it reads the case, arms it, drives its `host:`
## script through a fresh LingaraEmbed, finishes it, and writes one result
## line merging the server's verdict with its own. It reports every case:
## there is no skip. Run as
##   godot --headless --path godot/demo --script res://tests/ek_runner.gd

const Host := preload("res://tests/ek_host.gd")
const Match := preload("res://tests/ek_match.gd")
## make/godot.mk's EK_LANGS entry.
const LANG := "godot"

var _server := ""
var _http: HTTPRequest
## Every case's typed-signal checks, for test_signal_argument_types_hold.
var _checked := 0


func _initialize() -> void:
	_server = OS.get_environment("LINGARA_EMBED_SERVER_URL")
	var results := OS.get_environment("LINGARA_EMBED_RESULTS")
	if _server == "" or results == "":
		printerr("ek_runner: set LINGARA_EMBED_SERVER_URL and LINGARA_EMBED_RESULTS (conformance-server run sets both)")
		quit(2)
		return
	_http = HTTPRequest.new()
	root.add_child.call_deferred(_http)
	await process_frame
	var lines: Array = await test_every_ek_case_passes()
	var out := FileAccess.open(results, FileAccess.WRITE)
	for line: Dictionary in lines:
		out.store_line(JSON.stringify(line))
	out.close()
	test_signal_argument_types_hold()
	# A failing case is in the results, which the server judges; the exit
	# code says only that every case was run and reported, and so does the
	# success line run-script.sh waits for (ADR 30.10.26b D2).
	print("LINGARA_GODOT_SCRIPT_OK")
	quit(0)


## 1.10.26ab AC5: every EK case, through LingaraEmbed, none skipped.
func test_every_ek_case_passes() -> Array:
	var lines := []
	for id: String in _case_ids():
		lines.append(await _run_case(id))
	return lines


## 1.10.26ab AC4: every typed signal emitted during the run carried D3's
## Godot types. A mismatch is already a mismatch on its case's line; this
## says how many emissions were held, so a run that checked none is visible.
func test_signal_argument_types_hold() -> void:
	print("ek_runner: %d typed signal emissions checked against D3's types" % _checked)


## `<group>/<slug>.yaml` → `<group>.<slug>`, sorted.
func _case_ids() -> Array:
	var dir := OS.get_environment("LINGARA_EMBED_CASES")
	if dir == "":
		dir = ProjectSettings.globalize_path("res://").path_join("../../conformance/ek/cases")
	var ids := []
	for group: String in DirAccess.get_directories_at(dir):
		for file: String in DirAccess.get_files_at(dir.path_join(group)):
			if file.get_extension() == "yaml":
				ids.append("%s.%s" % [group, file.get_basename()])
	ids.sort()
	return ids


func _control(verb: String, id: String) -> String:
	return "%s/__conformance/%s/%s" % [_server, verb, id]


## [ok, body]: the parsed JSON body of one control call.
func _call(url: String, method: int, body := "") -> Array:
	var headers := PackedStringArray(["Content-Type: application/json"])
	if _http.request(url, headers, method, body) != OK:
		return [false, "the request did not start"]
	var reply: Array = await _http.request_completed
	var ok: bool = reply[0] == HTTPRequest.RESULT_SUCCESS and reply[1] < 300
	var text: String = (reply[3] as PackedByteArray).get_string_from_utf8()
	return [ok, JSON.parse_string(text) if text != "" else null, reply[1]]


func _fail(id: String, why: String) -> Dictionary:
	return {"case": id, "lang": LANG, "result": "fail", "mismatches": [why]}


## One case end to end. Any failure to reach the server is a mismatch, so
## the case is still reported.
func _run_case(id: String) -> Dictionary:
	var got: Array = await _call(_control("cases", id), HTTPClient.METHOD_GET)
	if not got[0] or not (got[1] is Dictionary):
		return _fail(id, "cannot read the case: %s" % [got.slice(1)])
	var case: Dictionary = Match.normalised(got[1])
	var mismatches := []
	var armed: Array = await _call(_control("arm", id), HTTPClient.METHOD_POST)
	if not armed[0]:
		mismatches.append("cannot arm: %s" % [armed.slice(1)])
	var host := Host.new(self, _server)
	await host.run(case)
	_checked += host.signals.checked
	mismatches.append_array(host.problems)
	var expect: Variant = case.get("expect")
	var want: Variant = expect.get("events") if expect is Dictionary else null
	if want is Array and not Match.same(host.matched, want):
		var saw := JSON.stringify(Match.without_nulls(host.matched))
		mismatches.append("%s: the host saw %s, expected %s" % [id, saw, JSON.stringify(want)])
	var verdict: Array = await _call(_control("finish", id), HTTPClient.METHOD_POST, "{}")
	if verdict[1] is Dictionary:
		mismatches.append_array(verdict[1].get("mismatches", []))
	else:
		mismatches.append("cannot finish: %s" % [verdict.slice(1)])
	var result := "pass" if mismatches.is_empty() else "fail"
	return {"case": id, "lang": LANG, "result": result, "mismatches": mismatches}
