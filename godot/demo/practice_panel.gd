extends VBoxContainer
## The practice panel (ADR 1.10.26ab D9): drawn from each snapshot
## practice_changed carries, answered with practice_choose, practice_answer
## or practice_self_grade, closed on practice_finished. It holds no state of
## its own beyond the session id: the snapshot is the whole truth.

signal finished(snapshot: Dictionary)

var session := -1
var phase := ""

@onready var _prompt: Label = $Prompt
@onready var _options: HBoxContainer = $Options
@onready var _answer: LineEdit = $Answer
@onready var _submit: Button = $Buttons/Submit
@onready var _reveal: Button = $Buttons/Reveal
@onready var _correct: Button = $Buttons/Correct
@onready var _incorrect: Button = $Buttons/Incorrect
@onready var _next: Button = $Buttons/Next
@onready var _result: Label = $Result


func _ready() -> void:
	var lingara := _lingara()
	lingara.practice_changed.connect(_on_changed)
	lingara.practice_finished.connect(_on_finished)
	_submit.pressed.connect(submit)
	_reveal.pressed.connect(func() -> void: lingara.practice_reveal(session))
	_correct.pressed.connect(grade.bind(true))
	_incorrect.pressed.connect(grade.bind(false))
	_next.pressed.connect(func() -> void: lingara.practice_next(session))
	visible = false


func _lingara() -> LingaraEmbed:
	return get_node("/root/Lingara") as LingaraEmbed


## A practice over a plan Lingara makes for this player, so its result reports.
func start(source: Dictionary) -> void:
	session = _lingara().practice_start(source)
	visible = session >= 0
	_prompt.text = "Making a plan…"


func choose(option: int) -> void:
	_lingara().practice_choose(session, option)


func submit() -> void:
	_lingara().practice_answer(session, _answer.text)
	_answer.text = ""


func grade(correct: bool) -> void:
	_lingara().practice_self_grade(session, correct)


func _on_changed(changed: int, snapshot: Dictionary) -> void:
	if changed != session:
		return
	phase = snapshot.get("phase", "")
	var question: Variant = snapshot.get("question")
	_prompt.text = question.get("prompt", "") if question is Dictionary else phase
	_draw_options(question.get("options") if question is Dictionary else null)
	var result: Variant = snapshot.get("last_result")
	_result.text = "" if not (result is Dictionary) else "%s — %s" % [result.get("expected", ""), result.get("explanation", "")]
	_answer.visible = phase == "unanswered" and _options.get_child_count() == 0
	_submit.visible = _answer.visible
	_reveal.visible = phase == "unanswered"
	_correct.visible = phase == "awaiting_self_grade"
	_incorrect.visible = _correct.visible
	_next.visible = phase in ["reviewed", "set_complete"]


func _draw_options(options: Variant) -> void:
	for child in _options.get_children():
		_options.remove_child(child)
		child.queue_free()
	if not (options is Array):
		return
	for i in options.size():
		var button := Button.new()
		button.name = "Option%d" % i
		button.text = options[i]
		button.pressed.connect(choose.bind(i))
		_options.add_child(button)


func _on_finished(ended: int, snapshot: Dictionary) -> void:
	if ended != session:
		return
	_prompt.text = "Practice %s." % snapshot.get("phase", "")
	finished.emit(snapshot)
