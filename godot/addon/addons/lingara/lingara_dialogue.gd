class_name LingaraDialogue
extends RefCounted
## Writes a `dialogue.v1` graph in GDScript (ADR 28.10.26g D1): NPCs, facts
## and checkpoints in the order they are added. Edge order matters: the core
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
const OUTCOMES := ["success", "failure", "abandoned"]
const CEFR := ["A1", "A2", "B1", "B2", "C1", "C2"]
## From the easiest: N5 is 1, the mistake this helper prevents.
const JLPT := ["N5", "N4", "N3", "N2", "N1"]
const TYPES := ["string", "number", "boolean"]

var _id: String
var _start: String
var _title := ""
var _pedagogy := "subtle"
var _npcs: Array[Npc] = []
var _facts: Array[Dictionary] = []
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

	## An edge to a checkpoint or to "success", "failure" or "abandoned";
	## `options` may hold "predicate" and "when".
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
		"schema": "dialogue.v1", "id": _id, "pedagogy": _pedagogy, "npcs": npcs,
		"facts": _facts, "start": _start, "checkpoints": checkpoints,
	}
	if _title != "":
		d["title"] = _title
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
	for check: Callable in [_ids, _unique, _targets, _disclosed, _defaults, _levels, _when_less_cycle]:
		var fault: String = check.call()
		if fault != "":
			return fault
	return ""


static func _npc_dictionary(n: Npc) -> Dictionary:
	var d := {"id": n.id, "name": n.name}
	if n.persona != "":
		d["persona"] = n.persona
	if not n.languages.is_empty():
		d["languages"] = n.languages
	return d


static func _is_id(value: String) -> bool:
	var pattern := RegEx.create_from_string("^[a-z][a-z0-9_]{0,47}$")
	return pattern.search(value) != null


## Every id in path order: [path, id].
func _all_ids() -> Array:
	var ids := [["id", _id], ["start", _start]]
	for i in _npcs.size():
		ids.append(["npcs[%d].id" % i, _npcs[i].id])
	for i in _facts.size():
		ids.append(["facts[%d].key" % i, _facts[i].key])
	for i in _checkpoints.size():
		ids.append(["checkpoints[%d].id" % i, _checkpoints[i].id])
		for j in _checkpoints[i].edges.size():
			ids.append(["checkpoints[%d].edges[%d].id" % [i, j], _checkpoints[i].edges[j].id])
	return ids


func _ids() -> String:
	for pair: Array in _all_ids():
		if not _is_id(pair[1]):
			return "%s: \"%s\" is not an id" % pair
	for i in _checkpoints.size():
		if _checkpoints[i].id in OUTCOMES:
			return "checkpoints[%d].id: \"%s\" is an outcome's name" % [i, _checkpoints[i].id]
	return ""


## Rule 1: unique within each list, and edge ids across the whole dialogue.
func _unique() -> String:
	var seen := {}
	for pair: Array in _all_ids().slice(2):
		var path: String = pair[0]
		var list := path.get_slice("[", 0) + ("edges" if path.contains("edges") else "")
		var key := "%s/%s" % [list, pair[1]]
		if seen.has(key):
			return "%s: repeats the id \"%s\"" % pair
		seen[key] = true
	return ""


## Rule 2: `start` and every non-outcome `to` name a checkpoint.
func _targets() -> String:
	var ids := _checkpoints.map(func(c: Checkpoint) -> String: return c.id)
	if not _start in ids:
		return "start: names no checkpoint: \"%s\"" % _start
	for i in _checkpoints.size():
		for j in _checkpoints[i].edges.size():
			var to: String = _checkpoints[i].edges[j].to
			if not to in OUTCOMES and not to in ids:
				return "checkpoints[%d].edges[%d].to: names no checkpoint or outcome: \"%s\"" % [i, j, to]
	return ""


## Rule 3, for `may_disclose`.
func _disclosed() -> String:
	var keys := _facts.map(func(f: Dictionary) -> String: return f.key)
	for i in _checkpoints.size():
		for k in _checkpoints[i].may_disclose.size():
			if not _checkpoints[i].may_disclose[k] in keys:
				return "checkpoints[%d].may_disclose[%d]: unknown fact \"%s\"" % [i, k, _checkpoints[i].may_disclose[k]]
	return ""


## Rule 5, and D2's builder range for a number.
func _defaults() -> String:
	for i in _facts.size():
		var f := _facts[i]
		if not f.type in TYPES:
			return "facts[%d].type: \"%s\" is not string, number or boolean" % [i, f.type]
		if not f.has("default"):
			continue
		var value: Variant = f.default
		var ok := false
		match f.type:
			"string":
				ok = value is String
			"boolean":
				ok = value is bool
			"number":
				ok = (value is int or value is float) and LingaraCanonicalJson.number(value) != ""
		if not ok:
			return "facts[%d].default: must match the fact's type, and a number the builders' range" % i
	return ""


func _levels() -> String:
	for i in _npcs.size():
		for j in _npcs[i].languages.size():
			var language := _npcs[i].languages[j]
			if language.speaking < 1 or language.speaking > 9 or language.comprehension < 1 or language.comprehension > 9:
				return "npcs[%d].languages[%d]: levels run from 1 to 9" % [i, j]
	return ""


## Rule 6: no cycle made only of edges without `when`.
func _when_less_cycle() -> String:
	var state := {}  # 1 on the path, 2 done
	for i in _checkpoints.size():
		if _visit(_checkpoints[i].id, state):
			return "checkpoints[%d]: closes a cycle of edges without `when`, which would loop with no turn" % i
	return ""


func _visit(id: String, state: Dictionary) -> bool:
	if state.has(id):
		return state[id] == 1
	state[id] = 1
	var found := false
	for c: Checkpoint in _checkpoints:
		if c.id != id:
			continue
		for e: Dictionary in c.edges:
			if not e.has("when") and not e.to in OUTCOMES and _visit(e.to, state):
				found = true
	state[id] = 2
	return found
