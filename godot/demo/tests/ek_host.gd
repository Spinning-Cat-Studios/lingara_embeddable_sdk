extends RefCounted
## One EK case's `host:` script through LingaraEmbed (ADR 1.10.26ab D10).
## Every step goes through the node's public methods, and every event is
## read off its `core_event` signal, which is already K1's JSON vocabulary.
## A method refused synchronously reports its `last_error()` as an `error`
## event ahead of the queue, as the Rust harness does.

const Match := preload("res://tests/ek_match.gd")
const Signals := preload("res://tests/ek_signals.gd")
## K1's flat methods and their argument order: these go through the method
## of that name, everything else through execute().
const FLAT := {
	"provide_token": ["token", "expires_at"],
	"provide_token_for": ["token", "expires_in_secs"],
	"token_unavailable": ["reason"],
	"set_token_endpoint": ["url", "headers"],
	"stream_pause": [],
	"stream_resume": [],
	"send_context": ["context"],
	"practice_start": ["source"],
	"practice_answer": ["session", "answer"],
	"practice_choose": ["session", "option"],
	"practice_reveal": ["session"],
	"practice_self_grade": ["session", "correct"],
	"practice_next": ["session"],
	"practice_finish": ["session"],
	"practice_snapshot": ["session"],
	"dialogue_open": ["config"],
	"dialogue_say": ["dialogue", "line"],
	"dialogue_set_level": ["dialogue", "level"],
	"dialogue_set_scene": ["dialogue", "scene"],
	"dialogue_cancel": ["dialogue"],
	"dialogue_close": ["dialogue"],
	"dialogue_snapshot": ["dialogue"],
}
const CLOSE_WITHIN_MS := 2000

var problems: Array[String] = []
var matched: Array = []
## The typed-signal checks, read by the runner.
var signals: Signals
var _tree: SceneTree
var _server: String
var _node: Node
var _queue: Array = []
var _local: Array = []
var _last_id: Variant = null


func _init(tree: SceneTree, server: String) -> void:
	_tree = tree
	_server = server
	signals = Signals.new()


## Runs every step; the first failing one ends the case.
func run(case: Dictionary) -> void:
	var steps: Array = case.get("host", [])
	for n in steps.size():
		var step: Dictionary = steps[n]
		var name: String = step.keys()[0]
		var problem: String = await _step(name, step[name])
		if problem != "":
			problems.append("%s: host[%d].%s: %s" % [case.get("id", ""), n, name, problem])
			break
	await _teardown()
	problems.append_array(signals.problems)


# One return, so gdlint's max-returns holds as steps are added.
func _step(name: String, body: Variant) -> String:
	var problem := "unknown step %s" % name
	match name:
		"start":
			problem = _start(body)
		"provide_token":
			problem = _provide(body)
		"command":
			problem = _command(body)
		"poll_until":
			problem = await _poll_until(body)
		"expect_no_event":
			problem = await _expect_no_event(body)
		"stop_polling":
			problem = await _stop_polling(body)
		"close":
			problem = await _close()
	return problem


## Every mapped property set explicitly, so the demo project's own
## lingara/* settings never reach a case.
func _start(body: Dictionary) -> String:
	var config: Dictionary = body.get("config", {})
	_node = ClassDB.instantiate("LingaraEmbed")
	_node.autostart = false
	_node.forward_logs = false
	_node.base_url = _server
	_node.token_url = ""
	_node.api_version = config.get("api_version", "")
	_node.stream_on_start = config.get("stream_on_start", true)
	_node.queue_capacity = config.get("queue_capacity", 1024)
	_node.log_level = config.get("log_level", "warn")
	_node.user_agent_suffix = config.get("user_agent_suffix", "")
	var extra := {}
	for key: String in ["event_types", "resume_cursor", "token_wait_timeout_ms", "stream_idle_timeout_ms"]:
		if config.get(key) != null:
			extra[key] = config[key]
	_node.extra_config = extra
	_node.core_event.connect(func(event: Dictionary) -> void: _queue.append(event))
	signals.watch(_node)
	_tree.root.add_child(_node)
	if not _node.start():
		return "start refused: %s" % _node.last_error()
	var token: Variant = body.get("initial_token", "none")
	if token is String and token != "none" and not _node.provide_token_for(token, 900):
		return "initial token refused: %s" % _node.last_error()
	return ""


func _refused() -> void:
	var error: Dictionary = _node.last_error()
	if not error.is_empty():
		_local.append({"type": "error", "error": error})


func _provide(body: Dictionary) -> String:
	if _node == null:
		return "no start step ran"
	if body.get("expires_at") != null:
		_node.provide_token(body["token"], body["expires_at"])
	elif body.get("expires_in_secs") != null:
		_node.provide_token_for(body["token"], body["expires_in_secs"])
	else:
		return "no expires_at or expires_in_secs"
	_refused()
	return ""


func _substituted(value: Variant) -> Variant:
	if value is String and value == "$last":
		return _last_id
	if value is Dictionary:
		var out := {}
		for key: Variant in value:
			out[key] = _substituted(value[key])
		return out
	if value is Array:
		return value.map(_substituted)
	return value


## A `Command` in its JSON shape: the flat method of that name with its
## fields as arguments, or execute().
func _command(body: Dictionary) -> String:
	if _node == null:
		return "no start step ran"
	var command: Dictionary = _substituted(body)
	var kind: String = command.get("type", "")
	if not FLAT.has(kind) or not _flat_arguments_present(command):
		_node.execute(command)
		_refused()
		return ""
	var answer: Variant = _node.callv(kind, _arguments(kind, command))
	_refused()
	if kind in ["practice_start", "dialogue_open"] and answer is int and answer >= 0:
		_last_id = answer
	return ""


## A command missing a field cannot be spelled as a call: execute() refuses
## it in K1's own words.
func _flat_arguments_present(command: Dictionary) -> bool:
	for field: String in FLAT[command["type"]]:
		if not command.has(field) and field not in ["headers", "scene"]:
			return false
	return true


func _arguments(kind: String, command: Dictionary) -> Array:
	var args := []
	for field: String in FLAT[kind]:
		var value: Variant = command.get(field)
		if field == "headers":
			var headers := {}
			for header: Dictionary in (value if value is Array else []):
				headers[header["name"]] = header["value"]
			value = headers
		elif field == "scene" and value == null:
			value = ""
		args.append(value)
	return args


func _next() -> Variant:
	if not _local.is_empty():
		return _local.pop_front()
	if not _queue.is_empty():
		return _queue.pop_front()
	return null


## Frames until an event matches, or `deadline` passes.
func _find(event: String, wanted: Variant, deadline: int) -> Variant:
	while Time.get_ticks_msec() < deadline:
		var seen: Variant = _next()
		if seen == null:
			await _tree.process_frame
		elif seen.get("type") == event and Match.contains(seen, wanted):
			return seen
	return null


func _poll_until(body: Dictionary) -> String:
	var within: int = body.get("within_ms", 0)
	var seen: Variant = await _find(body.get("event", ""), body.get("match"), Time.get_ticks_msec() + within)
	if seen == null:
		return "no %s event matching %s within %d ms" % [body.get("event"), body.get("match"), within]
	matched.append(seen)
	return ""


func _expect_no_event(body: Dictionary) -> String:
	var period: int = body.get("for_ms", 0)
	var seen: Variant = await _find(body.get("event", ""), body.get("match"), Time.get_ticks_msec() + period)
	return "" if seen == null else "an unexpected %s event: %s" % [body.get("event"), seen]


## EK9's backpressure: the node stops draining for `for_ms`.
func _stop_polling(body: Dictionary) -> String:
	if _node == null:
		return "no start step ran"
	_node.set_process(false)
	await _tree.create_timer(body.get("for_ms", 0) / 1000.0).timeout
	_node.set_process(true)
	return ""


func _close() -> String:
	if _node == null:
		return "no start step ran"
	_node.stop()
	if await _find("closed", null, Time.get_ticks_msec() + CLOSE_WITHIN_MS) == null:
		return "no closed event after close"
	await _tree.process_frame
	await _tree.process_frame
	var after: Variant = _next()
	return "" if after == null else "an event after closed: %s" % after


## A case that did not close is stopped, then freed; the library joins it.
func _teardown() -> void:
	if _node == null:
		return
	if _node.is_running():
		_node.stop()
		await _find("closed", null, Time.get_ticks_msec() + CLOSE_WITHIN_MS)
	_node.queue_free()
	await _tree.process_frame
	_node = null
