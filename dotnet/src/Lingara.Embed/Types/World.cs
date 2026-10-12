using System;
using System.Collections.Generic;

namespace Lingara.Embed
{
    /// <summary><c>world.context_changed</c> (K1 D6), bounded as the inbound door is.</summary>
    public sealed class WorldContext
    {
        public string Scene { get; init; } = string.Empty;

        public string SourceLang { get; init; } = string.Empty;

        public string TargetLang { get; init; } = string.Empty;

        public byte Level { get; init; }

        public Npc? Npc { get; init; }

        public IReadOnlyList<string> Tags { get; init; } = Array.Empty<string>();

        public bool Generate { get; init; }
    }

    public sealed class Npc
    {
        public string Name { get; init; } = string.Empty;

        public string? Persona { get; init; }
    }

    /// <summary>The <c>202</c>'s <c>reaction</c> to a world event (K1 D6).</summary>
    public sealed class Reaction
    {
        public string Status { get; init; } = string.Empty;

        public string? PlanId { get; init; }

        public string? PlanStatus { get; init; }

        public string? Code { get; init; }

        public string? Error { get; init; }
    }
}
