extends SceneTree
## A runtime error before quit(): Godot 4.5.2 on macOS hangs on it.


func _initialize() -> void:
	var missing: Node = null
	print(missing.get_name())
	print("LINGARA_GODOT_SCRIPT_OK")
	quit(0)
