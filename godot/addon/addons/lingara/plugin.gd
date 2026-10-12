@tool
extends EditorPlugin
## Adds the `Lingara` autoload when the plugin is enabled and removes it when
## disabled, and registers the five `lingara/*` project settings the node
## reads at construction (ADR 1.10.26ab D2, D6). Nothing secret has a
## setting: anything in ProjectSettings ships inside the exported game.

const AUTOLOAD := "Lingara"
const AUTOLOAD_SCENE := "res://addons/lingara/lingara_autoload.tscn"
const SETTINGS := {
	"lingara/base_url": "https://api.getlingara.com",
	"lingara/token_url": "",
	"lingara/api_version": "",
	"lingara/autostart": true,
	"lingara/stream_on_start": true,
}


func _enable_plugin() -> void:
	add_autoload_singleton(AUTOLOAD, AUTOLOAD_SCENE)
	_register_settings()


func _disable_plugin() -> void:
	remove_autoload_singleton(AUTOLOAD)


func _enter_tree() -> void:
	_register_settings()


func _register_settings() -> void:
	var changed := false
	for key: String in SETTINGS:
		var value: Variant = SETTINGS[key]
		if not ProjectSettings.has_setting(key):
			ProjectSettings.set_setting(key, value)
			changed = true
		ProjectSettings.set_initial_value(key, value)
		ProjectSettings.add_property_info({"name": key, "type": typeof(value)})
	if changed:
		ProjectSettings.save()
