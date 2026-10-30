class_name LingaraNpc
extends Node
## A talking character: one Lingara dialogue, opened when the core is
## running, with its turn signals filtered to that dialogue (ADR 1.10.26ab
## D5). Three NPCs in a scene are three LingaraNpc children, each wired to
## its own speech bubble. A convenience over the Lingara autoload: a game
## that wants another shape calls it directly.
##
## Graph mode (ADR 28.10.26g D6): set [member dialogue] (a built
## LingaraDialogue) or [member dialogue_file] (a dialogue.v1 file) and the NPC
## opens with `dialogue_load` instead; [member dialogue] wins when both are
## set, and [member dialogue_npc] names this NPC when the cast has more than
## one. The story arrives as [signal checkpoint_reached] and [signal ended].
## With neither set, it opens a free-form dialogue as before.

signal checkpoint_reached(checkpoint: String, edge: String, via: String)
signal ended(outcome: String)
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
## A dialogue.v1 file, for graph mode.
@export_file("*.json") var dialogue_file := ""
## The cast member this NPC is; required when the cast has more than one.
@export var dialogue_npc := ""

## A built dialogue, for graph mode; wins over [member dialogue_file].
var dialogue: LingaraDialogue

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
	lingara.checkpoint_reached.connect(_on_checkpoint_reached)
	lingara.dialogue_ended.connect(_on_dialogue_ended)
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


## Sets a fact the graph declared; an int or a float is a number. False
## before the dialogue is open, or when the core refuses it.
func set_fact(key: String, value: Variant) -> bool:
	if _dialogue < 0:
		return false
	return _lingara().dialogue_set_fact(_dialogue, key, value)


func _lingara() -> Variant:
	return get_node_or_null(AUTOLOAD_PATH) if is_inside_tree() else null


func _open() -> void:
	if dialogue != null or dialogue_file != "":
		_load()
		return
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


## Graph mode: the IR's text, from the built dialogue or the file, loaded
## with this NPC's learner settings. The core validates it.
func _load() -> void:
	var ir_json := dialogue.to_canonical_json() if dialogue != null else FileAccess.get_file_as_string(dialogue_file)
	var session := {"source_lang": source_lang, "target_lang": target_lang, "level": level, "window": initial_window}
	if dialogue_npc != "":
		session["npc"] = dialogue_npc
	if scene != "":
		session["scene"] = scene
	_dialogue = _lingara().dialogue_load({"type": "ir", "ir_json": ir_json}, session)
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


func _on_checkpoint_reached(loaded: int, checkpoint: String, edge: String, via: String) -> void:
	if loaded == _dialogue:
		checkpoint_reached.emit(checkpoint, edge, via)


func _on_dialogue_ended(loaded: int, outcome: String, _checkpoint: String, _edge: String) -> void:
	if loaded == _dialogue:
		ended.emit(outcome)


## An EmbedError carries a request, not a dialogue: filter by the last say's.
func _on_error(error: Dictionary) -> void:
	if _busy and error.get("request") == _request:
		_busy = false
		failed.emit(error)
