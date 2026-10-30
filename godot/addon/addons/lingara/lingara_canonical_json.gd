class_name LingaraCanonicalJson
extends RefCounted
## The dialogue IR's canonical form (ADR 28.10.26g D2), written by hand:
## RFC 8785 (JCS) for what LingaraDialogue holds. JSON.stringify is not used:
## it writes 2.0 for an integral float and does not sort keys by UTF-16 unit.
##
## Keys sort by UTF-16 code unit. Strings escape only ", \ and U+0000-U+001F
## (lowercase \u00xx where JSON has no short form). Numbers are the builders'
## range only: an integer to 2^53 - 1, or a non-integral number with at most
## four decimal places in 1e-4 <= |x| < 1e9. A Godot String holds code
## points, never a lone surrogate (it becomes U+FFFD), and cannot hold U+0000.

const MAX_SAFE_INTEGER := 9007199254740991


## `value` in canonical form, or "" with push_error on a number outside the
## builders' range or a value JSON cannot hold.
static func write(value: Variant) -> String:
	var out := PackedStringArray()
	if not _value(value, out):
		return ""
	return "".join(out)


## [method write] as UTF-8 bytes: what a load sends.
static func to_bytes(value: Variant) -> PackedByteArray:
	return write(value).to_utf8_buffer()


## [method write] after removing every member whose name starts "x-", at
## every depth: two IRs are the same dialogue when this is byte-equal.
static func comparison_form(value: Variant) -> String:
	return write(_strip(value))


## A number's canonical text, or "" outside the builders' range.
static func number(x: Variant) -> String:
	if x is int:
		return str(x) if absi(x) <= MAX_SAFE_INTEGER else ""
	var f: float = x
	if is_nan(f) or is_inf(f):
		return ""
	if f == floorf(f):
		return str(int(f)) if absf(f) <= MAX_SAFE_INTEGER else ""
	var four_places := roundf(f * 10000.0) / 10000.0 == f
	if absf(f) < 0.0001 or absf(f) >= 1e9 or not four_places:
		return ""
	# Exact in this range: the integer and the four places, trailing zeros cut.
	var scaled := int(roundf(absf(f) * 10000.0))
	var fraction := ("%04d" % (scaled % 10000)).rstrip("0")
	return "%s%d.%s" % ["-" if f < 0.0 else "", scaled / 10000, fraction]


## D2 rule 5: `text` as a quoted JSON string.
static func quote(text: String) -> String:
	var out := PackedStringArray(["\""])
	for i in text.length():
		var c := text.unicode_at(i)
		match c:
			0x22:
				out.append("\\\"")
			0x5C:
				out.append("\\\\")
			0x08:
				out.append("\\b")
			0x09:
				out.append("\\t")
			0x0A:
				out.append("\\n")
			0x0C:
				out.append("\\f")
			0x0D:
				out.append("\\r")
			_:
				out.append("\\u%04x" % c if c < 0x20 else char(c))
	out.append("\"")
	return "".join(out)


static func _value(value: Variant, out: PackedStringArray) -> bool:
	match typeof(value):
		TYPE_NIL:
			out.append("null")
		TYPE_BOOL:
			out.append("true" if value else "false")
		TYPE_INT, TYPE_FLOAT:
			var text := number(value)
			if text == "":
				push_error("LingaraCanonicalJson: %s is outside the builders' number range" % value)
				return false
			out.append(text)
		TYPE_STRING, TYPE_STRING_NAME:
			out.append(quote(String(value)))
		TYPE_ARRAY:
			return _array(value, out)
		TYPE_DICTIONARY:
			return _object(value, out)
		_:
			push_error("LingaraCanonicalJson: a %s is not JSON" % type_string(typeof(value)))
			return false
	return true


static func _array(items: Array, out: PackedStringArray) -> bool:
	out.append("[")
	for i in items.size():
		if i > 0:
			out.append(",")
		if not _value(items[i], out):
			return false
	out.append("]")
	return true


static func _object(members: Dictionary, out: PackedStringArray) -> bool:
	var keys: Array = members.keys()
	keys.sort_custom(func(a: Variant, b: Variant) -> bool: return _before(String(a), String(b)))
	out.append("{")
	for i in keys.size():
		if i > 0:
			out.append(",")
		out.append(quote(String(keys[i])))
		out.append(":")
		if not _value(members[keys[i]], out):
			return false
	out.append("}")
	return true


## JCS's order: by UTF-16 code unit, where an astral code point (a surrogate
## pair, D800-DBFF first) sorts before U+E000-U+FFFF.
static func _before(a: String, b: String) -> bool:
	var ua := _utf16(a)
	var ub := _utf16(b)
	for i in mini(ua.size(), ub.size()):
		if ua[i] != ub[i]:
			return ua[i] < ub[i]
	return ua.size() < ub.size()


static func _utf16(text: String) -> PackedInt32Array:
	var units := PackedInt32Array()
	for i in text.length():
		var c := text.unicode_at(i)
		if c > 0xFFFF:
			units.append(0xD800 + ((c - 0x10000) >> 10))
			units.append(0xDC00 + ((c - 0x10000) & 0x3FF))
		else:
			units.append(c)
	return units


static func _strip(value: Variant) -> Variant:
	if value is Array:
		return (value as Array).map(_strip)
	if not (value is Dictionary):
		return value
	var kept := {}
	for key: Variant in value:
		if not String(key).begins_with("x-"):
			kept[key] = _strip(value[key])
	return kept
