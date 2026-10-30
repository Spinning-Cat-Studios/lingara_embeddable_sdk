extends SceneTree
## snippets/player/gdscript/ carries exactly P1's four player keys and K2's
## dialogueGraph, no install, and no region fails P1's two guards (ADR
## 1.10.26ab D12, AC11; ADR 28.10.26g AC18).
##
## It lives outside snippets/ and builds every marker and forbidden string
## by concatenation: P1's sync vendors every marked file of a surface, and a
## test that spelled them literally would itself be a marked file and trip
## the guards. Run as
##   godot --headless --path godot/demo --script res://tests/snippet_regions_test.gd

const KEYS := ["dialogue", "dialogueGraph", "poll", "practice", "tokenSource"]
const PREFIX := "lingara" + ":"


func _initialize() -> void:
	var problems := test_four_keys_and_both_guards()
	for problem: String in problems:
		printerr("✗ ", problem)
	if problems.is_empty():
		print("✓ the gdscript snippet: every key, no install, both guards hold")
		print("LINGARA_GODOT_SCRIPT_OK")
	quit(1 if not problems.is_empty() else 0)


func _snippets() -> String:
	return ProjectSettings.globalize_path("res://").path_join("../../snippets/player/gdscript")


## Each key's regions, joined in file order.
func _regions(text: String) -> Dictionary:
	var begin := "# " + PREFIX + "begin "
	var end := "# " + PREFIX + "end"
	var regions := {}
	var key := ""
	for line: String in text.split("\n"):
		if line.begins_with(begin):
			key = line.trim_prefix(begin).strip_edges()
			regions[key] = regions.get(key, "")
		elif line.begins_with(end):
			key = ""
		elif key != "":
			regions[key] += line + "\n"
	return regions


## 1.10.26ab AC11.
func test_four_keys_and_both_guards() -> Array[String]:
	var problems: Array[String] = []
	var dir := _snippets()
	for file: String in DirAccess.get_files_at(dir):
		if file.begins_with("install" + "."):
			problems.append("%s: a store install is prose on P1's page, never a file" % file)
	var text := FileAccess.get_file_as_string(dir.path_join("lingara_snippet.gd"))
	var regions := _regions(text)
	var keys := regions.keys()
	keys.sort()
	if keys != KEYS:
		problems.append("the keys are %s, not %s" % [keys, KEYS])
	var token := RegEx.create_from_string("lgr" + "_[a-z]+_[A-Za-z0-9]{8,}")
	var forbidden := ["lgr" + "_cs_", "lgr" + "_whsec_", "lgr" + "_at_", "client" + "_secret", "LINGARA" + "_CLIENT_SECRET"]
	for key: String in regions:
		if token.search(regions[key]) != null:
			problems.append("region %s holds something shaped like a real token" % key)
		for word: String in forbidden:
			if regions[key].contains(word):
				problems.append("region %s names %s, which a player region never does" % [key, word])
	return problems
