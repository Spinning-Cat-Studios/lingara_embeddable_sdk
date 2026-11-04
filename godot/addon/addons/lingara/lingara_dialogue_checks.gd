extends RefCounted
## LingaraDialogue's structural checks (ADR 28.10.26g D5), under N2 §10's
## rule numbers: what needs no grammar. Each answers the first fault as
## "<json path>: <reason>", or "". The ending set is the declared endings,
## or v1's two when none is declared, plus "abandoned" (ADR 3.11.26g §11.1),
## the crate's rule, which AC22 holds this copy to.

const ABANDONED := "abandoned"
const TYPES := ["string", "number", "boolean"]


## The first structural fault of builder `d`, or "" for none.
static func first_fault(d: Variant) -> String:
	var ends: Array = ends_of(d)
	for fault: String in [
		_ids(d, ends), _unique(d), _targets(d, ends), _disclosed(d), _defaults(d), _levels(d), _when_less_cycle(d, ends)
	]:
		if fault != "":
			return fault
	return ""


## What an edge may end on: the declared endings plus "abandoned".
static func ends_of(d: Variant) -> Array:
	var ends: Array = d.declared_endings().map(func(e: Dictionary) -> String: return e.id)
	ends.append(ABANDONED)
	return ends


static func _is_id(value: String) -> bool:
	var pattern := RegEx.create_from_string("^[a-z][a-z0-9_]{0,47}$")
	return pattern.search(value) != null


## Every id in path order: [path, id].
static func _all_ids(d: Variant) -> Array:
	var ids := [["id", d._id], ["start", d._start]]
	for i in d._npcs.size():
		ids.append(["npcs[%d].id" % i, d._npcs[i].id])
	for i in d._facts.size():
		ids.append(["facts[%d].key" % i, d._facts[i].key])
	for i in d._endings.size():
		ids.append(["endings[%d].id" % i, d._endings[i].id])
	for i in d._checkpoints.size():
		ids.append(["checkpoints[%d].id" % i, d._checkpoints[i].id])
		for j in d._checkpoints[i].edges.size():
			ids.append(["checkpoints[%d].edges[%d].id" % [i, j], d._checkpoints[i].edges[j].id])
	return ids


static func _ids(d: Variant, ends: Array) -> String:
	for pair: Array in _all_ids(d):
		if not _is_id(pair[1]):
			return "%s: \"%s\" is not an id" % pair
	for i in d._endings.size():
		if d._endings[i].id == ABANDONED:
			return "endings[%d].id: \"abandoned\" is built in, so it is never declared" % i
	for i in d._checkpoints.size():
		if d._checkpoints[i].id in ends:
			return "checkpoints[%d].id: \"%s\" is an ending's id" % [i, d._checkpoints[i].id]
	return ""


## Rule 1: unique within each list, and edge ids across the whole dialogue.
static func _unique(d: Variant) -> String:
	var seen := {}
	for pair: Array in _all_ids(d).slice(2):
		var path: String = pair[0]
		var list := path.get_slice("[", 0) + ("edges" if path.contains("edges") else "")
		var key := "%s/%s" % [list, pair[1]]
		if seen.has(key):
			return "%s: repeats the id \"%s\"" % pair
		seen[key] = true
	return ""


## Rule 2: `start` names a checkpoint, and every `to` a checkpoint or an ending.
static func _targets(d: Variant, ends: Array) -> String:
	var ids: Array = d._checkpoints.map(func(c: Variant) -> String: return c.id)
	if not d._start in ids:
		return "start: names no checkpoint: \"%s\"" % d._start
	for i in d._checkpoints.size():
		for j in d._checkpoints[i].edges.size():
			var to: String = d._checkpoints[i].edges[j].to
			if not to in ends and not to in ids:
				return "checkpoints[%d].edges[%d].to: names no checkpoint or ending: \"%s\"" % [i, j, to]
	return ""


## Rule 3, for `may_disclose`.
static func _disclosed(d: Variant) -> String:
	var keys: Array = d._facts.map(func(f: Dictionary) -> String: return f.key)
	for i in d._checkpoints.size():
		var disclosed: Array = d._checkpoints[i].may_disclose
		for k in disclosed.size():
			if not disclosed[k] in keys:
				return "checkpoints[%d].may_disclose[%d]: unknown fact \"%s\"" % [i, k, disclosed[k]]
	return ""


## Rule 5, and D2's builder range for a number.
static func _defaults(d: Variant) -> String:
	for i in d._facts.size():
		var f: Dictionary = d._facts[i]
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


static func _levels(d: Variant) -> String:
	for i in d._npcs.size():
		for j in d._npcs[i].languages.size():
			var language: Dictionary = d._npcs[i].languages[j]
			if language.speaking < 1 or language.speaking > 9 or language.comprehension < 1 or language.comprehension > 9:
				return "npcs[%d].languages[%d]: levels run from 1 to 9" % [i, j]
	return ""


## Rule 6: no cycle made only of edges without `when`; an edge to an ending closes none.
static func _when_less_cycle(d: Variant, ends: Array) -> String:
	var state := {}  # 1 on the path, 2 done
	for i in d._checkpoints.size():
		if _visit(d, d._checkpoints[i].id, state, ends):
			return "checkpoints[%d]: closes a cycle of edges without `when`, which would loop with no turn" % i
	return ""


static func _visit(d: Variant, id: String, state: Dictionary, ends: Array) -> bool:
	if state.has(id):
		return state[id] == 1
	state[id] = 1
	var found := false
	for c: Variant in d._checkpoints:
		if c.id != id:
			continue
		for e: Dictionary in c.edges:
			if not e.has("when") and not e.to in ends and _visit(d, e.to, state, ends):
				found = true
	state[id] = 2
	return found
