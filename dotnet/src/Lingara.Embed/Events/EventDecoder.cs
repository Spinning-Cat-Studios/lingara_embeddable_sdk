using System;
using System.Collections.Generic;
using System.Text;
using Lingara.Embed.Json;

namespace Lingara.Embed.Events
{
    /// <summary>
    /// A static table from each K1 event <c>type</c> to its own decode function
    /// (ADR 1.10.26aa D8): no reflection, and no function past CA1502's budget.
    /// </summary>
    internal static class EventDecoder
    {
        private static readonly Dictionary<string, Func<JsonValue, EmbedEvent>> Table = new Dictionary<string, Func<JsonValue, EmbedEvent>>
        {
            ["token_needed"] = v => new EmbedEvent.TokenNeeded
            {
                Request = new RequestId(v.Long("request")),
                Reason = Words.TokenReasons.Parse(v.Str("reason")),
            },
            ["stream"] = v => new EmbedEvent.StreamChanged { State = TypeDecoders.Stream(v.Obj("state")), Cursor = v.OptStr("cursor") },
            ["lingara"] = v => new EmbedEvent.LingaraReceived { Event = TypeDecoders.LingaraEvent(v.Obj("event")) },
            ["world_sent"] = v => new EmbedEvent.WorldSent
            {
                Request = new RequestId(v.Long("request")),
                EventId = v.Str("event_id"),
                Reaction = TypeDecoders.Reaction(v["reaction"]),
            },
            ["practice"] = v => new EmbedEvent.Practice
            {
                Session = new SessionId(v.Long("session")),
                Snapshot = TypeDecoders.Practice(v.Obj("snapshot")),
            },
            ["npc_turn"] = v => new EmbedEvent.NpcTurn
            {
                Dialogue = new DialogueId(v.Long("dialogue")),
                Request = new RequestId(v.Long("request")),
                Update = TypeDecoders.TurnUpdate(v.Obj("update")),
            },
            ["checkpoint_reached"] = v => new EmbedEvent.CheckpointReached
            {
                Dialogue = new DialogueId(v.Long("dialogue")),
                Checkpoint = v.Str("checkpoint"),
                Edge = v.OptStr("edge"),
                Via = GraphWords.CrossedVias.Parse(v.Str("via")),
            },
            ["dialogue_ended"] = v => new EmbedEvent.DialogueEnded
            {
                Dialogue = new DialogueId(v.Long("dialogue")),
                Outcome = GraphWords.Outcomes.Parse(v.Str("outcome")),
                Checkpoint = v.Str("checkpoint"),
                Edge = v.OptStr("edge"),
            },
            ["dialogue_loaded"] = v => new EmbedEvent.DialogueLoaded
            {
                Dialogue = new DialogueId(v.Long("dialogue")),
                StoreId = v.Str("dialogue_id"),
                Rev = v.Long("rev"),
            },
            ["dialogue_load_failed"] = v => new EmbedEvent.DialogueLoadFailed
            {
                Dialogue = new DialogueId(v.Long("dialogue")),
                Error = TypeDecoders.Error(v.Obj("error")),
            },
            ["error"] = v => new EmbedEvent.Failed { Error = TypeDecoders.Error(v.Obj("error")) },
            ["log"] = v => new EmbedEvent.Log
            {
                Level = Words.LogLevels.Parse(v.Str("level")),
                Target = v.Str("target"),
                Message = v.Str("message"),
            },
            ["update_available"] = v => new EmbedEvent.UpdateAvailable
            {
                Latest = v.OptStr("latest"),
                Deprecated = v.Bool("deprecated"),
                Message = v.OptStr("message"),
            },
            ["closed"] = v => new EmbedEvent.Closed { Reason = Words.CloseReasons.Parse(v.Str("reason")) },
        };

        /// <summary>Every <c>type</c> with a row, for the schema coverage test.</summary>
        public static IEnumerable<string> Types => Table.Keys;

        /// <exception cref="JsonFormatException">Not an event in K1's shape.</exception>
        public static EmbedEvent Decode(ReadOnlySpan<byte> utf8, bool keepRaw)
        {
            var value = JsonReader.Parse(utf8);
            var type = value.Str("type");
            var json = keepRaw || !Table.ContainsKey(type) ? Encoding.UTF8.GetString(utf8.ToArray()) : null;
            var evt = Table.TryGetValue(type, out var decode) ? decode(value) : new EmbedEvent.Unknown(type, json!);
            evt.RawJson = keepRaw ? json : null;
            return evt;
        }
    }
}
