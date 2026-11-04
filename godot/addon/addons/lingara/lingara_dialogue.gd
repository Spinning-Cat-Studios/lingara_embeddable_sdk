class_name LingaraDialogue
extends RefCounted
## Writes a dialogue graph in GDScript (ADR 28.10.26g D1): NPCs, facts,
## endings and checkpoints in the order they are added. It writes
## `dialogue.v1`, or `dialogue.v2` once [method ending] declares an ending v1
## cannot say (ADR 3.11.26g §11.5). Edge order matters: the core
## crosses the first open edge with no `when`, and the judge takes the first
## match. A predicate is the core's text ("trust >= 0.4"), never a Callable.
##
## [method to_canonical_json] checks the structure first (D5) and returns ""
## with push_error on a fault. The predicate grammar, the length bounds and
## the byte cap are the core's: `dialogue_load` and `dialogue_validate` run
## them, with JSON paths.
##
## Levels run 1-9. [method cefr] and [method jlpt] map labels offline and do
## not check the language; HSK, TOPIK and General labels are their number.
## Under "subtle" the NPC speaks at the lower of its level and the learner's.

## The top of the scale: what an NPC with no languages is treated as.
const NATIVE := 9
## The ending no dialogue declares: the game closed the story unfinished.
const ABANDONED := "abandoned"
## What a dialogue that declares no ending ends on, besides ABANDONED.
const V1_ENDINGS := ["success", "failure"]
const CEFR := ["A1", "A2", "B1", "B2", "C1", "C2"]
## From the easiest: N5 is 1, the mistake this helper prevents.
const JLPT := ["N5", "N4", "N3", "N2", "N1"]
const Checks := preload("lingara_dialogue_checks.gd")

var _id: String
var _start: String
var _title := ""
var _pedagogy := "subtle"
var _npcs: Array[Npc] = []
var _facts: Array[Dictionary] = []
var _endings: Array[Dictionary] = []
var _checkpoints: Array[Checkpoint] = []


## A member of the cast and its N1 language profile.
class Npc:
	extends RefCounted
	var id: String
	var name: String
	var persona: String
	var languages: Array[Dictionary] = []

	func _init(npc_id: String, npc_name: String, npc_persona: String) -> void:
		id = npc_id
		name = npc_name
		persona = npc_persona

	## A language at speaking and comprehension levels 1-9.
	func speaks(code: String, speaking: int, comprehension: int) -> Npc:
		languages.append({"code": code, "speaking": speaking, "comprehension": comprehension})
		return self


## A beat of the story: its disclosed facts and its edges, in order.
class Checkpoint:
	extends RefCounted
	var id: String
	var goal: String
	var may_disclose: Array[String] = []
	var edges: Array[Dictionary] = []

	func _init(checkpoint_id: String, checkpoint_goal: String) -> void:
		id = checkpoint_id
		goal = checkpoint_goal

	## A declared fact key the NPC may voice here.
	func discloses(key: String) -> Checkpoint:
		may_disclose.append(key)
		return self

	## An edge to a checkpoint, to a declared ending, or to "abandoned" (with
	## no ending declared, also "success" or "failure"); `options` may hold
	## "predicate" and "when".
	func edge(edge_id: String, to: String, options := {}) -> Checkpoint:
		var e := {"id": edge_id, "to": to}
		for key: String in ["predicate", "when"]:
			if options.has(key):
				e[key] = options[key]
		edges.append(e)
		return self


func _init(id: String, start: String) -> void:
	_id = id
	_start = start


static func cefr(label: String) -> int:
	return _level(CEFR, label, "CEFR")


static func jlpt(label: String) -> int:
	return _level(JLPT, label, "JLPT")


static func _level(labels: Array, label: String, scale: String) -> int:
	var i := labels.find(label)
	if i < 0:
		push_error("LingaraDialogue: \"%s\" is not a %s level (%s)" % [label, scale, ", ".join(labels)])
	return i + 1 if i >= 0 else -1


func title(text: String) -> LingaraDialogue:
	_title = text
	return self


## "subtle" (the default) or "none".
func pedagogy(value: String) -> LingaraDialogue:
	_pedagogy = value
	return self


func npc(id: String, npc_name: String, persona := "") -> Npc:
	var n := Npc.new(id, npc_name, persona)
	_npcs.append(n)
	return n


## A fact of `type` "string", "number" or "boolean"; `default` null is none.
func fact(key: String, type: String, default: Variant = null) -> LingaraDialogue:
	var f := {"key": key, "type": type}
	if default != null:
		f["default"] = default
	_facts.append(f)
	return self


## A named ending (ADR 3.11.26g §11): once one is declared, the dialogue
## ends only on the endings it declares, plus "abandoned".
func ending(id: String, label := "") -> LingaraDialogue:
	var e := {"id": id}
	if label != "":
		e["label"] = label
	_endings.append(e)
	return self


## The declared endings, or v1's two when none was declared.
func declared_endings() -> Array[Dictionary]:
	if not _endings.is_empty():
		return _endings
	var v1: Array[Dictionary] = []
	for id: String in V1_ENDINGS:
		v1.append({"id": id})
	return v1


## The write rule, a copy of the crate's `version::write_schema` (ADR
## 3.11.26g §2) for what this builder holds: "dialogue.v1" unless an ending
## is not success / failure or has a label. AC22 pins its bytes to Rust's.
func write_schema() -> String:
	for e: Dictionary in declared_endings():
		if e.has("label") or not e.id in V1_ENDINGS:
			return "dialogue.v2"
	return "dialogue.v1"


func checkpoint(id: String, goal: String) -> Checkpoint:
	var c := Checkpoint.new(id, goal)
	_checkpoints.append(c)
	return c


## The dialogue in normal form (D2 rule 1), as JSON-ready values.
func to_dictionary() -> Dictionary:
	var npcs := _npcs.map(func(n: Npc) -> Dictionary: return _npc_dictionary(n))
	var checkpoints := _checkpoints.map(
		func(c: Checkpoint) -> Dictionary: return {"id": c.id, "goal": c.goal, "may_disclose": c.may_disclose, "edges": c.edges}
	)
	var d := {
		"schema": write_schema(), "id": _id, "pedagogy": _pedagogy, "npcs": npcs,
		"facts": _facts, "start": _start, "checkpoints": checkpoints,
	}
	if _title != "":
		d["title"] = _title
	if d.schema == "dialogue.v2":
		d["kind"] = "static"
		d["endings"] = declared_endings()
	return d


## The canonical text `dialogue_load` sends, or "" with push_error on a
## structural fault (D5).
func to_canonical_json() -> String:
	var fault := structural_fault()
	if fault != "":
		push_error("LingaraDialogue: " + fault)
		return ""
	return LingaraCanonicalJson.write(to_dictionary())


## The first structural fault, as "<json path>: <reason>", or "" for none.
func structural_fault() -> String:
	return Checks.first_fault(self)


static func _npc_dictionary(n: Npc) -> Dictionary:
	var d := {"id": n.id, "name": n.name}
	if n.persona != "":
		d["persona"] = n.persona
	if not n.languages.is_empty():
		d["languages"] = n.languages
	return d
