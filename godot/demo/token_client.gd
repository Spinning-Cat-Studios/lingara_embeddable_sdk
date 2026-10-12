extends Node
## The signal round trip (ADR 1.10.26ab D4, D9): each token_needed is answered
## with a POST to the game's own token route, here K2's demo token server, and
## its two keys go to provide_token. Any failure calls token_unavailable, so
## the core's waiting calls fail rather than time out. This is the path an
## integrator with an existing login copies; the demo holds no secret and no
## token in any file.

const DEFAULT_URL := "http://127.0.0.1:8787/token"


func _ready() -> void:
	_lingara().token_needed.connect(_on_token_needed)


func _lingara() -> LingaraEmbed:
	return get_node("/root/Lingara") as LingaraEmbed


func _url() -> String:
	var url := OS.get_environment("LINGARA_DEMO_TOKEN_URL")
	return url if url != "" else DEFAULT_URL


func _on_token_needed(_request: int, _reason: String) -> void:
	var http := HTTPRequest.new()
	add_child(http)
	var headers := PackedStringArray(["Content-Type: application/json"])
	if http.request(_url(), headers, HTTPClient.METHOD_POST, "{}") != OK:
		http.queue_free()
		_lingara().token_unavailable("the token request did not start")
		return
	var reply: Array = await http.request_completed
	http.queue_free()
	var body: Variant = JSON.parse_string((reply[3] as PackedByteArray).get_string_from_utf8())
	if reply[0] == HTTPRequest.RESULT_SUCCESS and reply[1] == 200 and body is Dictionary and body.has("token"):
		_lingara().provide_token(body["token"], body["expires_at"])
	else:
		_lingara().token_unavailable("the token server answered %d" % reply[1])
