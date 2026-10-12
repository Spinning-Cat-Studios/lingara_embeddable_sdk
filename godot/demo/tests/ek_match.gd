extends RefCounted
## How the EK harness compares JSON (ADR 1.10.26ab D10, K1's CONTRACT.md):
## a `poll_until` event must *contain* the case's `match`, compared after
## dropping nulls. Every number in a parsed case is a float, because
## JSON.parse_string reads them so, and every number K1 sends is an int, so
## `normalised` turns a whole float into an int before anything compares.


## A whole float becomes an int, deep.
static func normalised(value: Variant) -> Variant:
	match typeof(value):
		TYPE_FLOAT:
			var f: float = value
			return int(f) if is_finite(f) and f == floorf(f) and absf(f) < 9.0e18 else f
		TYPE_DICTIONARY:
			var out := {}
			for key: Variant in value:
				out[key] = normalised(value[key])
			return out
		TYPE_ARRAY:
			var items := []
			for item: Variant in value:
				items.append(normalised(item))
			return items
	return value


static func without_nulls(value: Variant) -> Variant:
	match typeof(value):
		TYPE_DICTIONARY:
			var out := {}
			for key: Variant in value:
				if value[key] != null:
					out[key] = without_nulls(value[key])
			return out
		TYPE_ARRAY:
			var items := []
			for item: Variant in value:
				items.append(without_nulls(item))
			return items
	return value


## An object matches when each of its non-null keys does; an array when it
## has the same length and each element matches; anything else when equal.
static func contains(actual: Variant, wanted: Variant) -> bool:
	if wanted == null:
		return true
	if typeof(wanted) == TYPE_DICTIONARY:
		if typeof(actual) != TYPE_DICTIONARY:
			return false
		for key: Variant in wanted:
			if wanted[key] != null and not (actual.has(key) and contains(actual[key], wanted[key])):
				return false
		return true
	if typeof(wanted) == TYPE_ARRAY:
		if typeof(actual) != TYPE_ARRAY or actual.size() != wanted.size():
			return false
		for i in wanted.size():
			if not contains(actual[i], wanted[i]):
				return false
		return true
	return typeof(actual) == typeof(wanted) and actual == wanted


## JSON-equal after dropping nulls.
static func same(a: Variant, b: Variant) -> bool:
	var x: Variant = without_nulls(a)
	var y: Variant = without_nulls(b)
	return contains(x, y) and contains(y, x)
