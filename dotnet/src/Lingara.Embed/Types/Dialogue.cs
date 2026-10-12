using System;
using System.Collections.Generic;

namespace Lingara.Embed
{
    /// <summary>An NPC dialogue's request, held between turns (K1 D8).</summary>
    public sealed class DialogueConfig
    {
        public Npc Npc { get; init; } = new Npc();

        public string? Scene { get; init; }

        public string SourceLang { get; init; } = string.Empty;

        public string TargetLang { get; init; } = string.Empty;

        public byte Level { get; init; }

        public IReadOnlyList<WindowEntry> Window { get; init; } = Array.Empty<WindowEntry>();
    }

    public sealed class WindowEntry
    {
        public Speaker Speaker { get; init; }

        public string Text { get; init; } = string.Empty;
    }

    public sealed class DialogueSnapshot
    {
        public DialoguePhase Phase { get; init; }

        public Npc Npc { get; init; } = new Npc();

        public string? Scene { get; init; }

        public byte Level { get; init; }

        public IReadOnlyList<WindowEntry> Window { get; init; } = Array.Empty<WindowEntry>();
    }

    /// <summary>One step of an NPC's reply (K1 D8).</summary>
    public abstract class NpcTurnUpdate
    {
        private NpcTurnUpdate()
        {
        }

        /// <summary>The reply's headers are in.</summary>
        public sealed class Opened : NpcTurnUpdate
        {
        }

        public sealed class Fragment : NpcTurnUpdate
        {
            public string Text { get; init; } = string.Empty;
        }

        public sealed class Notice : NpcTurnUpdate
        {
            public string Code { get; init; } = string.Empty;

            public string Message { get; init; } = string.Empty;
        }

        /// <summary>The full reply, uncut.</summary>
        public sealed class Finished : NpcTurnUpdate
        {
            public string Text { get; init; } = string.Empty;
        }

        public sealed class Cancelled : NpcTurnUpdate
        {
        }

        /// <summary>An update a newer core added.</summary>
        public sealed class Unknown : NpcTurnUpdate
        {
            public string Type { get; init; } = string.Empty;
        }
    }
}
