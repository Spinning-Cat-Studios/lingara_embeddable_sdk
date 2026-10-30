extends SceneTree
## An analyser error gdlint does not see: an identifier that does not exist.
## Godot 4.5.2 on macOS exits 0 on it, --check-only included.


func _initialize() -> void:
	print(this_identifier_does_not_exist)
	print("LINGARA_GODOT_SCRIPT_OK")
	quit(0)
