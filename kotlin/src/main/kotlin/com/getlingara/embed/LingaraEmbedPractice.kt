@file:JvmName("LingaraEmbedPractice")

package com.getlingara.embed

// The practice commands (ADR 1.10.26ae D3). Each answers the session's
// snapshot in the same call, so a game shows the graded answer this frame.

/** Starts a practice session; its id. Its snapshots also arrive as `Practice` events. */
@Throws(EmbedException::class)
public fun LingaraEmbed.practiceStart(source: PracticeSource): Long = handle.practiceStart(source)

/** The session's snapshot, or null for an unknown session. */
@Throws(EmbedException::class)
public fun LingaraEmbed.practiceSnapshot(session: Long): PracticeSnapshot? = handle.practiceSnapshot(session)

@Throws(EmbedException::class)
public fun LingaraEmbed.practiceAnswer(
    session: Long,
    answer: String,
): PracticeSnapshot = handle.practiceAnswer(session, answer)

/** Answers a multiple-choice question with its option index. */
@Throws(EmbedException::class)
public fun LingaraEmbed.practiceChoose(
    session: Long,
    option: Long,
): PracticeSnapshot = handle.practiceChoose(session, option)

@Throws(EmbedException::class)
public fun LingaraEmbed.practiceReveal(session: Long): PracticeSnapshot = handle.practiceReveal(session)

/** The learner's own grade for a self-graded question. */
@Throws(EmbedException::class)
public fun LingaraEmbed.practiceSelfGrade(
    session: Long,
    correct: Boolean,
): PracticeSnapshot = handle.practiceSelfGrade(session, correct)

@Throws(EmbedException::class)
public fun LingaraEmbed.practiceNext(session: Long): PracticeSnapshot = handle.practiceNext(session)

/** Ends the session early; the rest is reported as skipped. */
@Throws(EmbedException::class)
public fun LingaraEmbed.practiceFinish(session: Long): PracticeSnapshot = handle.practiceFinish(session)
