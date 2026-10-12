extends Control
## The Godot demo (ADR 1.10.26ab D9): a market square with two LingaraNpcs,
## Mei (Mandarin, level 2) and Professor Tanaka (Japanese, level 6).
## Clicking a stall selects its NPC without walking; the player types a line
## (or takes a preset), and the selected NPC answers in its speech bubble.
## Walking to the other stall sends one world context. Practice asks Lingara
## for a plan in the selected NPC's language and level.
##
## It starts the core itself, after its configuration is in place: the
## project's lingara/autostart is false, so no request leaves before this.

const PRESETS := {
	"Mei": ["你好！", "饺子多少钱？", "谢谢！"],
	"Professor Tanaka": ["こんにちは、先生。", "今日は何を勉強しますか？", "ありがとうございます。"],
}
const STALLS := {"Mei": "Mei's tea and dumpling stall", "Professor Tanaka": "the Professor's bookstall"}

var selected: LingaraNpc
var _here := "Mei"

@onready var mei: LingaraNpc = $Mei
@onready var tanaka: LingaraNpc = $Tanaka
@onready var line: LineEdit = $UI/Talk/Line
@onready var send: Button = $UI/Talk/Send
@onready var level: HSlider = $UI/Level/Slider
@onready var practice: Node = $UI/PracticePanel
@onready var _bubbles := {mei: $UI/Bubbles/Mei, tanaka: $UI/Bubbles/Tanaka}


func _ready() -> void:
	for npc: LingaraNpc in [mei, tanaka]:
		npc.thinking.connect(_on_thinking.bind(npc))
		npc.fragment.connect(_on_fragment.bind(npc))
		npc.replied.connect(_on_replied.bind(npc))
		npc.failed.connect(_on_failed.bind(npc))
	$UI/Stalls/Mei.pressed.connect(select.bind(mei))
	$UI/Stalls/Tanaka.pressed.connect(select.bind(tanaka))
	send.pressed.connect(say)
	line.text_submitted.connect(func(_text: String) -> void: say())
	level.value_changed.connect(func(value: float) -> void: selected.level = int(value))
	$UI/Walk.pressed.connect(walk)
	$UI/Practice.pressed.connect(start_practice)
	for i in 3:
		$UI/Presets.get_child(i).pressed.connect(_preset.bind(i))
	select(mei)
	var lingara := get_node("/root/Lingara") as LingaraEmbed
	var api := OS.get_environment("LINGARA_DEMO_API_URL")
	if api != "":
		lingara.base_url = api
	lingara.start()


func _process(_delta: float) -> void:
	send.disabled = selected == null or selected.busy


## Selects an NPC without walking.
func select(npc: LingaraNpc) -> void:
	selected = npc
	$UI/Selected.text = "Talking to %s" % npc.npc_name
	level.set_value_no_signal(npc.level)
	for i in 3:
		$UI/Presets.get_child(i).text = PRESETS[npc.npc_name][i]


func say() -> void:
	if line.text.strip_edges() != "" and selected.say(line.text):
		line.text = ""


## One inbound world.context_changed per walk, never per frame.
func walk() -> void:
	_here = "Professor Tanaka" if _here == "Mei" else "Mei"
	var npc: LingaraNpc = tanaka if _here == "Professor Tanaka" else mei
	var context := {
		"scene": STALLS[_here],
		"source_lang": "en",
		"target_lang": npc.target_lang,
		"level": npc.level,
		"npc": {"name": npc.npc_name},
	}
	(get_node("/root/Lingara") as LingaraEmbed).send_context(context)
	$UI/Selected.text = "You walk to %s" % STALLS[_here]


func start_practice() -> void:
	practice.start({
		"type": "request",
		"topic": "buying food at a market",
		"source_lang": "en",
		"target_lang": selected.target_lang,
		"level": selected.level,
	})


func _preset(i: int) -> void:
	line.text = PRESETS[selected.npc_name][i]


func _on_thinking(npc: LingaraNpc) -> void:
	_bubbles[npc].text = "…"


func _on_fragment(text: String, npc: LingaraNpc) -> void:
	var bubble: Label = _bubbles[npc]
	bubble.text = text if bubble.text == "…" else bubble.text + text


func _on_replied(text: String, npc: LingaraNpc) -> void:
	_bubbles[npc].text = text
	print("%s: %s" % [npc.npc_name, text])


func _on_failed(error: Dictionary, npc: LingaraNpc) -> void:
	_bubbles[npc].text = "(%s)" % error.get("kind", "error")
