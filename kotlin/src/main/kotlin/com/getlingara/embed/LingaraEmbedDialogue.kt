@file:JvmName("LingaraEmbedDialogue")

package com.getlingara.embed

// The NPC dialogue commands (ADR 1.10.26ae D3). A reply streams back as
// `NpcTurn` events, fragment by fragment.

/** Opens a conversation with an NPC; its id. */
@Throws(EmbedException::class)
public fun LingaraEmbed.dialogueOpen(config: DialogueConfig): Long = handle.dialogueOpen(config)

/** Sends the player's line; the request id its `NpcTurn` events carry. */
@Throws(EmbedException::class)
public fun LingaraEmbed.dialogueSay(
    dialogue: Long,
    line: String,
): Long = handle.dialogueSay(dialogue, line)

/** The NPC's level, 1–9, for the next turn. */
@Throws(EmbedException::class)
public fun LingaraEmbed.dialogueSetLevel(
    dialogue: Long,
    level: Int,
): Unit = handle.dialogueSetLevel(dialogue, level)

@Throws(EmbedException::class)
public fun LingaraEmbed.dialogueSetScene(
    dialogue: Long,
    scene: String?,
): Unit = handle.dialogueSetScene(dialogue, scene)

/** Cancels the turn in flight. */
@Throws(EmbedException::class)
public fun LingaraEmbed.dialogueCancel(dialogue: Long): Unit = handle.dialogueCancel(dialogue)

@Throws(EmbedException::class)
public fun LingaraEmbed.dialogueClose(dialogue: Long): Unit = handle.dialogueClose(dialogue)

/** The conversation's snapshot, or null for an unknown dialogue. */
@Throws(EmbedException::class)
public fun LingaraEmbed.dialogueSnapshot(dialogue: Long): DialogueSnapshot? = handle.dialogueSnapshot(dialogue)
