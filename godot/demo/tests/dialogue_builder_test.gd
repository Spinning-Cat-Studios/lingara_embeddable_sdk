extends SceneTree
## The GDScript builder and canonical writer (ADR 28.10.26g D1-D5) against
## conformance/vectors/dialogue/, which is outside this project's res://, so
## its absolute path comes as a user argument. Run as
##   godot --headless --path godot/demo --script res://tests/dialogue_builder_test.gd \
##     -- --vectors=<repo>/conformance/vectors/dialogue
## Every fault below push_errors by design; only the ✗ lines are failures.

var _vectors := ""


func _initialize() -> void:
	for arg: String in OS.get_cmdline_user_args():
		if arg.begins_with("--vectors="):
			_vectors = arg.trim_prefix("--vectors=")
	var problems: Array[String] = []
	if _vectors == "":
		problems.append("pass -- --vectors=<repo>/conformance/vectors/dialogue")
	else:
		problems.append_array(test_the_worked_dialogues_match_the_golden_bytes())
		problems.append_array(test_the_corpus_and_integral_numbers())
		problems.append_array(test_levels_and_structural_refusals())
	for problem: String in problems:
		printerr("✗ ", problem)
	if problems.is_empty():
		print("✓ the GDScript builder writes the golden bytes, passes both fixtures, and refuses D5's faults")
		print("LINGARA_GODOT_SCRIPT_OK")
	quit(1 if not problems.is_empty() else 0)


func _bytes(name: String) -> PackedByteArray:
	return FileAccess.get_file_as_bytes(_vectors.path_join(name))


func _parsed(name: String) -> Variant:
	return JSON.parse_string(_bytes(name).get_string_from_utf8())


## N2's two-keepers dialogue.
static func two_keepers() -> LingaraDialogue:
	var d := LingaraDialogue.new("night_market_keepers", "suspicious_of_player")
	d.title("The two keepers").pedagogy("none")
	d.npc("old_wei", "Old Wei", "a retired ferryman who distrusts strangers") \
		.speaks("zh", LingaraDialogue.NATIVE, LingaraDialogue.NATIVE)
	d.fact("trust", "number", 0)
	d.fact("eastern_gate", "string")
	d.checkpoint("suspicious_of_player", "Size the stranger up; warm to them only if they explain why they are here") \
		.edge("asks_about_brother", "reveal_gate", {"predicate": "trust >= 0.4", "when": "the player has asked about their missing brother"}) \
		.edge("turns_hostile", "failure", {"when": "the player threatens or insults Old Wei"})
	d.checkpoint("reveal_gate", "Tell the player the eastern gate is guarded, reluctantly") \
		.discloses("eastern_gate") \
		.edge("thanks", "success", {"when": "the player thanks Old Wei or says goodbye"})
	return d


## D1's worked dialogue, extended as D4 describes.
static func missing_brother() -> LingaraDialogue:
	var d := LingaraDialogue.new("missing_brother", "introduction")
	d.title("The missing brother").pedagogy("none")
	d.npc("elena", "Elena", "a farmer who distrusts soldiers") \
		.speaks("es", LingaraDialogue.NATIVE, LingaraDialogue.NATIVE) \
		.speaks("en", LingaraDialogue.cefr("A2"), LingaraDialogue.cefr("B1"))
	d.npc("tomas", "Tomás", "Elena's neighbour, a retired soldier") \
		.speaks("es", LingaraDialogue.NATIVE, LingaraDialogue.NATIVE) \
		.speaks("en", 6, 7)
	d.fact("trust", "number", 0)
	d.fact("suspicion", "number", 0.4)
	d.fact("alarm_raised", "boolean", false)
	d.fact("eastern_gate", "string")
	d.checkpoint("introduction", "Elena is wary and asks who the player is") \
		.edge("alarm", "failure", {"predicate": "alarm_raised"}) \
		.edge("explained", "ask_about_brother", {"when": "the player says who they are and why they are here"})
	d.checkpoint("ask_about_brother", "Admit she saw soldiers ride toward \"the old mill\"; reveal the gate only if she trusts the player") \
		.discloses("eastern_gate") \
		.edge("gate", "success", {"predicate": "trust >= 0.4", "when": "the player asks where soldiers might take a prisoner"}) \
		.edge("neighbour", "ask_tomas", {"when": "the player asks who else saw the soldiers"}) \
		.edge("leaves", "abandoned", {"when": "the player walks away"})
	d.checkpoint("ask_tomas", "Tomás says the soldiers spoke of the road to 長安") \
		.edge("suspicious", "failure", {"predicate": "suspicion > 0.5"}) \
		.edge("back_to_elena", "ask_about_brother", {"when": "the player returns to Elena with what Tomás said"})
	return d


## 28.10.26g AC8.
func test_the_worked_dialogues_match_the_golden_bytes() -> Array[String]:
	var problems: Array[String] = []
	var cases := {"two-keepers.comparison.json": two_keepers(), "missing-brother.comparison.json": missing_brother()}
	for name: String in cases:
		var written: PackedByteArray = (cases[name] as LingaraDialogue).to_canonical_json().to_utf8_buffer()
		if written != _bytes(name):
			problems.append("%s: wrote %s" % [name, written.get_string_from_utf8()])
	return problems


## 28.10.26g AC9. A Godot String cannot hold U+0000, so the one pair whose
## input is U+0000 is a string no GDScript builder can write, and is skipped.
func test_the_corpus_and_integral_numbers() -> Array[String]:
	var problems: Array[String] = []
	var strings: Dictionary = _parsed("canonical-strings.json")
	for pair: Dictionary in strings.strings:
		if pair.canonical == "\"\\u0000\"":
			continue
		if LingaraCanonicalJson.quote(pair.input) != pair.canonical:
			problems.append("string %s: wrote %s" % [pair.canonical, LingaraCanonicalJson.quote(pair.input)])
	for pair: Dictionary in strings["keys"]:
		if LingaraCanonicalJson.write(pair.input) != pair.canonical:
			problems.append("keys: wrote %s" % LingaraCanonicalJson.write(pair.input))
	var numbers: Dictionary = _parsed("canonical-numbers.json")
	for pair: Dictionary in numbers.builder:
		var text: String = pair.input
		var x: Variant = text.to_int() if text.is_valid_int() else text.to_float()
		if LingaraCanonicalJson.number(x) != pair.canonical:
			problems.append("number %s: wrote %s" % [text, LingaraCanonicalJson.number(x)])
	# A level or an integral default as an int, never 2.0, whatever type it was held in.
	var integral := LingaraCanonicalJson.write({"level": 2, "default": 2.0, "zero": -0.0})
	if integral != "{\"default\":2,\"level\":2,\"zero\":0}":
		problems.append("integral numbers: wrote %s" % integral)
	# The comparison form of the canonical two-keepers is the comparison vector.
	var canonical: Variant = _parsed("two-keepers.canonical.json")
	if LingaraCanonicalJson.comparison_form(canonical).to_utf8_buffer() != _bytes("two-keepers.comparison.json"):
		problems.append("comparison form: wrote %s" % LingaraCanonicalJson.comparison_form(canonical))
	return problems


## A one-checkpoint dialogue whose fault is `change`'s.
static func _small(change: Callable) -> LingaraDialogue:
	var d := LingaraDialogue.new("d", "a")
	d.npc("n", "N")
	d.fact("x", "number")
	change.call(d)
	return d


## 28.10.26g AC10.
func test_levels_and_structural_refusals() -> Array[String]:
	var problems: Array[String] = []
	var labels: Dictionary = _parsed("level-labels.json")
	for scale: String in ["cefr", "jlpt"]:
		var map := LingaraDialogue.cefr if scale == "cefr" else LingaraDialogue.jlpt
		for label: String in labels[scale]:
			if map.call(label) != int(labels[scale][label]):
				problems.append("%s %s is %d" % [scale, label, map.call(label)])
	if LingaraDialogue.NATIVE != int(labels.native) or LingaraDialogue.cefr("D1") != -1 or LingaraDialogue.jlpt("N6") != -1:
		problems.append("NATIVE, or an unknown label's -1")
	var judged := {"when": "w"}
	var faults := {
		"id": func(_d: LingaraDialogue) -> void: _d._id = "Bad-Id",
		"facts[1].key": func(d: LingaraDialogue) -> void: d.fact("x", "boolean"),
		"checkpoints[1].edges[0].id": func(d: LingaraDialogue) -> void: d.checkpoint("b", "B").edge("e", "success", judged),
		"checkpoints[0].edges[1].to": func(d: LingaraDialogue) -> void: d._checkpoints[0].edge("f", "nowhere", judged),
		"checkpoints[1].id": func(d: LingaraDialogue) -> void: d.checkpoint("success", "S"),
		"checkpoints[0].may_disclose[0]": func(d: LingaraDialogue) -> void: d._checkpoints[0].discloses("secret"),
		"npcs[1].languages[0]": func(d: LingaraDialogue) -> void: d.npc("m", "M").speaks("en", 10, 1),
		"facts[1].default": func(d: LingaraDialogue) -> void: d.fact("y", "number", true),
	}
	for path: String in faults:
		problems.append_array(_refused(path, faults[path]))
	for x: Variant in [0.00001, 1000000000.5, 0.12345, 9007199254740992]:
		problems.append_array(_refused("facts[1].default", func(d: LingaraDialogue) -> void: d.fact("y", "number", x)))
	var loop := func(d: LingaraDialogue) -> void:
		d.checkpoint("b", "B").edge("back", "a", {"predicate": "x < 1"})
	problems.append_array(_refused("checkpoints[0]", loop))
	var judged_loop := _small(func(d: LingaraDialogue) -> void:
		d.checkpoint("a", "A").edge("forth", "b", {"predicate": "x > 1"})
		d.checkpoint("b", "B").edge("back", "a", {"when": "the player asks to go back"}))
	if judged_loop.to_canonical_json() == "":
		problems.append("a cycle through one judged edge was refused: %s" % judged_loop.structural_fault())
	return problems


## `change` over a valid small dialogue (an edge `e` to success from `a`;
## for the cycle case, `a` crosses to `b` with no `when`).
func _refused(path: String, change: Callable) -> Array[String]:
	var d := _small(func(base: LingaraDialogue) -> void:
		var forth := {"predicate": "x > 1"} if path == "checkpoints[0]" else {"when": "w"}
		base.checkpoint("a", "A").edge("e", "b" if path == "checkpoints[0]" else "success", forth)
		change.call(base))
	var fault := d.structural_fault()
	if not fault.begins_with(path + ":") or d.to_canonical_json() != "":
		return ["expected a refusal at %s, got \"%s\"" % [path, fault]]
	return []
