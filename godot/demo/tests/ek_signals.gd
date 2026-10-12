extends RefCounted
## D3's Godot types, held on every typed signal the EK run emits (ADR
## 1.10.26ab AC4). `cargo test` cannot reach the Variant layer; this can.

## Each typed signal's argument types, in D3's order.
const TYPES := {
	"started": [],
	"token_needed": [TYPE_INT, TYPE_STRING],
	"stream_changed": [TYPE_DICTIONARY, TYPE_STRING],
	"event_received": [TYPE_STRING, TYPE_STRING, TYPE_STRING, TYPE_DICTIONARY],
	"world_sent": [TYPE_INT, TYPE_STRING, TYPE_DICTIONARY],
	"practice_changed": [TYPE_INT, TYPE_DICTIONARY],
	"practice_finished": [TYPE_INT, TYPE_DICTIONARY],
	"npc_turn_opened": [TYPE_INT, TYPE_INT],
	"npc_fragment": [TYPE_INT, TYPE_STRING],
	"npc_notice": [TYPE_INT, TYPE_STRING, TYPE_STRING],
	"npc_replied": [TYPE_INT, TYPE_STRING],
	"npc_turn_cancelled": [TYPE_INT],
	"error_raised": [TYPE_DICTIONARY],
	"log_received": [TYPE_STRING, TYPE_STRING, TYPE_STRING],
	"closed": [TYPE_STRING],
	"core_event": [TYPE_DICTIONARY],
}

var problems: Array[String] = []
## How many typed emissions were checked, across the run.
var checked := 0


## Connects a checker to every typed signal. Each takes its arguments as
## one Array, so one function serves every arity.
func watch(node: Node) -> void:
	for name: String in TYPES:
		node.connect(name, _checker(name))


func _checker(name: String) -> Callable:
	var arity: int = TYPES[name].size()
	match arity:
		0:
			return func() -> void: _check(name, [])
		1:
			return func(a: Variant) -> void: _check(name, [a])
		2:
			return func(a: Variant, b: Variant) -> void: _check(name, [a, b])
		3:
			return func(a: Variant, b: Variant, c: Variant) -> void: _check(name, [a, b, c])
	return func(a: Variant, b: Variant, c: Variant, d: Variant) -> void: _check(name, [a, b, c, d])


func _check(name: String, args: Array) -> void:
	checked += 1
	var want: Array = TYPES[name]
	for i in want.size():
		if typeof(args[i]) != want[i]:
			problems.append("%s argument %d is %s, not %s" % [name, i, type_string(typeof(args[i])), type_string(want[i])])
