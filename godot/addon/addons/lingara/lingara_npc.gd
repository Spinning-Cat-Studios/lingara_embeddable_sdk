class_name LingaraNpc
extends Node
## A talking character: one Lingara dialogue, opened when the core is
## running, with its turn signals filtered to that dialogue (ADR 1.10.26ab
## D5). Three NPCs in a scene are three LingaraNpc children, each wired to
## its own speech bubble. A convenience over the Lingara autoload: a game
## that wants another shape calls it directly.

signal thinking
signal fragment(text: String)
signal noticed(code: String, message: String)
signal replied(text: String)
signal cancelled
signal failed(error: Dictionary)

const AUTOLOAD_PATH := "/root/Lingara"

@export var npc_name := ""
@export var persona := ""
@export var target_lang := ""
@export var source_lang := "en"
@export_range(1, 9) var level := 1:
	set = _set_level
@export var scene := "":
	set = _set_scene

## Seeds the dialogue's window: a saved `dialogue_snapshot(id).window`.
var initial_window: Array = []
## True while a turn is in flight. Read-only.
var busy: bool:
	get:
		return _busy

var _dialogue := -1
var _request := -1
var _busy := false


func _ready() -> void:
	var lingara: Variant = _lingara()
	if lingara == null:
		push_warning("LingaraNpc: no Lingara autoload at %s; enable the Lingara plugin" % AUTOLOAD_PATH)
		return
	lingara.started.connect(_open)
	lingara.closed.connect(_on_closed)
	lingara.npc_turn_opened.connect(_on_opened)
	lingara.npc_fragment.connect(_on_fragment)
	lingara.npc_notice.connect(_on_notice)
	lingara.npc_replied.connect(_on_replied)
	lingara.npc_turn_cancelled.connect(_on_cancelled)
	lingara.error_raised.connect(_on_error)
	if lingara.is_running():
		_open()


func _exit_tree() -> void:
	var lingara: Variant = _lingara()
	if _dialogue >= 0 and lingara != null:
		lingara.dialogue_close(_dialogue)
	_dialogue = -1


## Sends one line. False while busy or before the dialogue is open: a turn is
## never queued, because each one is billed.
func say(line: String) -> bool:
	if _busy or _dialogue < 0:
		return false
	var request: int = _lingara().dialogue_say(_dialogue, line)
	if request < 0:
		return false
	_request = request
	_busy = true
	return true


## The dialogue's id, or -1 while it is not open.
func dialogue_id() -> int:
	return _dialogue


func _lingara() -> Variant:
	return get_node_or_null(AUTOLOAD_PATH) if is_inside_tree() else null


func _open() -> void:
	var config := {
		"npc": {"name": npc_name},
		"source_lang": source_lang,
		"target_lang": target_lang,
		"level": level,
		"window": initial_window,
	}
	if persona != "":
		config["npc"]["persona"] = persona
	if scene != "":
		config["scene"] = scene
	_dialogue = _lingara().dialogue_open(config)
	_busy = false


func _set_level(value: int) -> void:
	level = value
	if _dialogue >= 0:
		_lingara().dialogue_set_level(_dialogue, value)


func _set_scene(value: String) -> void:
	scene = value
	if _dialogue >= 0:
		_lingara().dialogue_set_scene(_dialogue, value)


func _on_closed(_reason: String) -> void:
	_dialogue = -1
	_busy = false


func _on_opened(dialogue: int, _request_id: int) -> void:
	if dialogue == _dialogue:
		thinking.emit()


func _on_fragment(dialogue: int, text: String) -> void:
	if dialogue == _dialogue:
		fragment.emit(text)


func _on_notice(dialogue: int, code: String, message: String) -> void:
	if dialogue == _dialogue:
		noticed.emit(code, message)


func _on_replied(dialogue: int, text: String) -> void:
	if dialogue == _dialogue:
		_busy = false
		replied.emit(text)


func _on_cancelled(dialogue: int) -> void:
	if dialogue == _dialogue:
		_busy = false
		cancelled.emit()


## An EmbedError carries a request, not a dialogue: filter by the last say's.
func _on_error(error: Dictionary) -> void:
	if _busy and error.get("request") == _request:
		_busy = false
		failed.emit(error)
