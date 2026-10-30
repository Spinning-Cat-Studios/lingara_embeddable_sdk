extends SceneTree
## The demo with no window (ADR 1.10.26ab D10, AC6), in K2's shape:
## godot/ci/run-headless-demo.sh starts `conformance-server serve` over
## tests/cases/ and K2's token server with --fake, both on ports of their own
## choosing, and runs this with LINGARA_DEMO_API_URL and
## LINGARA_DEMO_TOKEN_URL. It drives main.tscn through its own controls, with
## no walk: a typed line to Mei (level 2), a click on Tanaka's stall and a
## line to him (level 6), Mei again with her slider raised to 3, then a
## practice answered to the end. A `pass` verdict means exactly one
## world.practice_completed reached the server.

const CASE := "demo.talk-and-practise"
const WAIT_MS := 10000

var _main: Node
var _http: HTTPRequest
var _replies: Array = []
var _finished: Variant = null
## The names of the NPCs a story signal came from.
var _story := {}


func _initialize() -> void:
	var problems: Array = await test_demo_talks_at_two_levels_and_practises()
	for problem: String in problems:
		printerr("✗ ", problem)
	if problems.is_empty():
		print("✓ the demo talked at levels 2, 6 and 3, practised to reported, and %s passed" % CASE)
		print("LINGARA_GODOT_SCRIPT_OK")
	quit(1 if not problems.is_empty() else 0)


## 1.10.26ab AC6.
func test_demo_talks_at_two_levels_and_practises() -> Array:
	var api := OS.get_environment("LINGARA_DEMO_API_URL")
	if api == "" or OS.get_environment("LINGARA_DEMO_TOKEN_URL") == "":
		return ["run through godot/ci/run-headless-demo.sh (it sets LINGARA_DEMO_API_URL and LINGARA_DEMO_TOKEN_URL)"]
	_http = HTTPRequest.new()
	root.add_child.call_deferred(_http)
	await process_frame
	if not await _control(api, "arm"):
		return ["cannot arm %s" % CASE]
	_autoload()
	_main = (load("res://main.tscn") as PackedScene).instantiate()
	root.add_child(_main)
	var problems := []
	var elena := _graph_npc()
	if not await _until(func() -> bool: return _story.has(elena.name)):
		problems.append("the graph-mode NPC reached no checkpoint")
	var levels := [await _talk("你好！"), await _talk_to_tanaka(), await _talk_to_mei_at(3)]
	if levels != [2, 6, 3]:
		problems.append("turns at levels %s, not [2, 6, 3]" % [levels])
	if _replies.size() != 3:
		problems.append("%d npc_replied, not 3" % _replies.size())
	var phase := await _practise()
	if phase != "reported":
		problems.append("the practice ended %s, not reported" % phase)
	var verdict: Variant = await _control(api, "finish")
	if not (verdict is Dictionary) or verdict.get("result") != "pass":
		problems.append("%s: %s" % [CASE, verdict])
	var heard: Array = _story.keys()
	if heard != [elena.name]:
		problems.append("story signals from %s, not only %s" % [heard, elena.name])
	return problems


## ADR 28.10.26g AC13: a second LingaraNpc, in graph mode over the worked
## dialogue's IR, beside the free-form ones main.tscn holds. Every NPC's story
## signals are logged with its name, so the log shows the graph NPC's lines
## and none from the free-form NPCs.
func _graph_npc() -> LingaraNpc:
	for npc: Node in _main.find_children("*", "LingaraNpc"):
		_listen(npc)
	var elena := LingaraNpc.new()
	elena.name = "elena"
	elena.dialogue_file = ProjectSettings.globalize_path("res://").path_join("../../snippets/dialogue/missing-brother.json")
	elena.dialogue_npc = "elena"
	elena.target_lang = "es"
	elena.level = 2
	_listen(elena)
	_main.add_child(elena)
	return elena


func _listen(npc: LingaraNpc) -> void:
	npc.checkpoint_reached.connect(func(checkpoint: String, edge: String, via: String) -> void:
		print("checkpoint_reached %s: %s (edge %s, via %s)" % [npc.name, checkpoint, edge, via])
		_story[npc.name] = true)
	npc.ended.connect(func(outcome: String) -> void:
		print("ended %s: %s" % [npc.name, outcome])
		_story[npc.name] = true)


## The autoload, supplied by path when --script mode has not: main.gd and
## LingaraNpc reach it as /root/Lingara, never by its global name.
func _autoload() -> void:
	var lingara := root.get_node_or_null("Lingara")
	if lingara == null:
		lingara = (load("res://addons/lingara/lingara_autoload.tscn") as PackedScene).instantiate()
		lingara.name = "Lingara"
		root.add_child(lingara)
	lingara.npc_replied.connect(func(_dialogue: int, text: String) -> void: _replies.append(text))
	lingara.practice_finished.connect(func(_session: int, snapshot: Dictionary) -> void: _finished = snapshot)


func _control(api: String, verb: String) -> Variant:
	var url := "%s/__conformance/%s/%s" % [api, verb, CASE]
	var headers := PackedStringArray(["Content-Type: application/json"])
	if _http.request(url, headers, HTTPClient.METHOD_POST, "{}") != OK:
		return null
	var reply: Array = await _http.request_completed
	if reply[0] != HTTPRequest.RESULT_SUCCESS or reply[1] >= 300:
		return null
	var text := (reply[3] as PackedByteArray).get_string_from_utf8()
	return JSON.parse_string(text) if text != "" else true


## Frames until `done` holds, for at most WAIT_MS of real I/O on loopback.
func _until(done: Callable) -> bool:
	var deadline := Time.get_ticks_msec() + WAIT_MS
	while Time.get_ticks_msec() < deadline:
		if done.call():
			return true
		await process_frame
	return false


func _press(path: String) -> void:
	(_main.get_node(path) as BaseButton).pressed.emit()
	await process_frame


## Types a line to the selected NPC and waits for its reply; the level the
## turn was sent at.
func _talk(text: String) -> int:
	var npc: LingaraNpc = _main.selected
	await _until(func() -> bool: return npc.dialogue_id() >= 0 and not npc.busy)
	var level: int = npc.level
	var before := _replies.size()
	(_main.get_node("UI/Talk/Line") as LineEdit).text = text
	await _press("UI/Talk/Send")
	await _until(func() -> bool: return _replies.size() > before)
	return level


func _talk_to_tanaka() -> int:
	await _press("UI/Stalls/Tanaka")
	return await _talk("こんにちは、先生。")


func _talk_to_mei_at(level: int) -> int:
	await _press("UI/Stalls/Mei")
	(_main.get_node("UI/Level/Slider") as HSlider).value = level
	return await _talk("饺子多少钱？")


func _phase_is(phase: String) -> Callable:
	var panel: Node = _main.get_node("UI/PracticePanel")
	return func() -> bool: return panel.phase == phase


## The plan's two questions: the multiple choice answered with its second
## option, the free response answered and graded correct, then on to the
## report.
func _practise() -> String:
	await _press("UI/Practice")
	await _until(_phase_is("unanswered"))
	await _press("UI/PracticePanel/Options/Option1")
	await _until(_phase_is("reviewed"))
	await _press("UI/PracticePanel/Buttons/Next")
	await _until(_phase_is("unanswered"))
	(_main.get_node("UI/PracticePanel/Answer") as LineEdit).text = "饺子多少钱？"
	await _press("UI/PracticePanel/Buttons/Submit")
	await _until(_phase_is("awaiting_self_grade"))
	await _press("UI/PracticePanel/Buttons/Correct")
	await _press("UI/PracticePanel/Buttons/Next")
	await _press("UI/PracticePanel/Buttons/Next")
	await _until(func() -> bool: return _finished != null)
	return _finished.get("phase", "") if _finished is Dictionary else "unfinished"
