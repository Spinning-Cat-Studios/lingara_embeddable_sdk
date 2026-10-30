extends Node
# The GDScript snippets the docs show (ADR 1.10.26ab D12): a game with the
# Lingara addon enabled gets its tokens from its own server, listens to the
# core, talks to an NPC and practises. CI parses it with --check-only; its
# regions are what the docs page reads.

var _session := -1

# lingara:begin poll
# The Lingara autoload the plugin adds, reached by path and typed, so every
# call and signal below is checked against LingaraEmbed.
@onready var lingara: LingaraEmbed = get_node("/root/Lingara")
# lingara:end
@onready var mei: LingaraNpc = $Mei
@onready var bubble: Label = $Bubble


# lingara:begin tokenSource
# The common case: set Project Settings > lingara/token_url to your server's
# token route, which answers exactly {"token", "expires_at"}. Lingara POSTs to
# it on its own thread whenever it needs a token. If the route needs the
# player's session, set it at run time instead (never in ProjectSettings,
# which ships inside the export):
func use_my_session(session_cookie: String) -> void:
	lingara.set_token_endpoint("https://game.example.com/lingara-token", {"cookie": session_cookie})


# Or answer each token_needed yourself, with whatever login your game has.
func _on_token_needed(_request: int, _reason: String) -> void:
	var http := HTTPRequest.new()
	add_child(http)
	http.request("https://game.example.com/lingara-token", [], HTTPClient.METHOD_POST, "{}")
	var reply: Array = await http.request_completed
	http.queue_free()
	var body: Variant = JSON.parse_string((reply[3] as PackedByteArray).get_string_from_utf8())
	if reply[1] == 200 and body is Dictionary:
		lingara.provide_token(body["token"], body["expires_at"])
	else:
		lingara.token_unavailable("the token route answered %d" % reply[1])
# lingara:end


# lingara:begin poll
# Enable the plugin (Project > Project Settings > Plugins): it adds the
# Lingara autoload, which drains the core once per frame and emits one signal
# per event. Connect the signals your game needs; you never call drain.
func _ready() -> void:
	lingara.token_needed.connect(_on_token_needed)
	lingara.event_received.connect(_on_event)
	lingara.error_raised.connect(func(error: Dictionary) -> void: push_warning(error.message))
	lingara.practice_changed.connect(_on_practice_changed)
	lingara.practice_finished.connect(_on_practice_finished)
	mei.replied.connect(_on_mei_replied)


func _on_event(_id: String, event_type: String, _created_at: String, data: Dictionary) -> void:
	if event_type == "lesson_plan.ready":
		print("a plan is ready: ", data.get("plan_id"))
# lingara:end


# lingara:begin dialogue
# A LingaraNpc child per character: npc_name, target_lang and level in the
# inspector. say() returns false while a turn is in flight (turns are never
# queued, because each one is billed).
func talk_to_mei(line: String) -> void:
	if mei.say(line):
		bubble.text = "…"


func _on_mei_replied(text: String) -> void:
	bubble.text = text
# lingara:end


# lingara:begin dialogueGraph
# A story as a graph: checkpoints joined by edges. An edge's "predicate" is a
# game condition Lingara's core evaluates; its "when" is a conversational one
# the server's judge reads. Two NPCs share this graph, each with their own
# fluency (1-9; cefr("A2") is 2, NATIVE is 9).
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


# A LingaraNpc in graph mode: set its dialogue (or dialogue_file to a saved
# IR) and dialogue_npc before it enters the tree; it loads with
# dialogue_load, and say() talks to her as to any NPC.
func add_elena() -> LingaraNpc:
	var elena := LingaraNpc.new()
	elena.dialogue = missing_brother()
	elena.dialogue_npc = "elena"
	elena.target_lang = "es"
	elena.level = 2
	elena.checkpoint_reached.connect(func(checkpoint: String, _edge: String, _via: String) -> void: print("now at ", checkpoint))
	elena.ended.connect(func(outcome: String) -> void: print("the story ended: ", outcome))
	add_child(elena)
	return elena


# The game sets facts; a predicate that reads one may cross an edge.
func raise_the_alarm(elena: LingaraNpc) -> void:
	elena.set_fact("alarm_raised", true)
# lingara:end


# lingara:begin practice
# A practice session over a plan Lingara makes for this player. Each answer
# method returns the new snapshot and also emits practice_changed.
func start_practice() -> void:
	_session = lingara.practice_start({
		"type": "request",
		"topic": "buying food at a market",
		"source_lang": "en",
		"target_lang": "zh",
		"level": 3,
	})


# A demo plan Lingara refuses a report on, so practise one with report false.
func practise_held_plan(plan_id: String) -> void:
	_session = lingara.practice_start({"type": "plan", "plan_id": plan_id, "report": false})


func _on_practice_changed(session: int, snapshot: Dictionary) -> void:
	if session != _session or snapshot.phase != "unanswered":
		return
	var question: Dictionary = snapshot.question
	if question.options != null:
		lingara.practice_choose(session, 0)
	else:
		lingara.practice_answer(session, "your learner's answer")


func _on_practice_finished(_session_id: int, snapshot: Dictionary) -> void:
	print("practice ended: ", snapshot.phase)
# lingara:end
