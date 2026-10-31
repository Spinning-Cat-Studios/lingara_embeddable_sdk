using System;
using System.Collections.Generic;

namespace Lingara.Embed
{
    /// <summary>One header the Http token source sends. The value is a secret: a game session is a credential too.</summary>
    public sealed class Header
    {
        public Header(string name, string value)
        {
            Name = name;
            Value = value;
        }

        public string Name { get; }

        public string Value { get; }

        /// <summary>Never the value: a header can be logged without leaking it.</summary>
        public override string ToString() => $"{Name}: [REDACTED]";
    }

    /// <summary>
    /// The handoff's optional <c>update_available</c> (ADR 30.10.26p): a newer
    /// or better version than this build's <c>EmbedVersion</c>. Pass it to
    /// <c>ProvideToken</c> when your game fetches the handoff itself.
    /// </summary>
    public sealed class UpdateAvailable
    {
        /// <summary>The version now serving, or null when no published version serves.</summary>
        public string? Latest { get; init; }

        public bool Deprecated { get; init; }

        /// <summary>The developer's message, verbatim, or null.</summary>
        public string? Message { get; init; }
    }

    /// <summary>
    /// K1's Http token source (<c>TokenSourceConfig::Http</c>, D6): the core POSTs
    /// <c>{}</c> to <see cref="Url"/> with <see cref="Headers"/> and reads the handoff body itself.
    /// </summary>
    public sealed class TokenEndpoint
    {
        public string Url { get; init; } = string.Empty;

        public IReadOnlyList<Header> Headers { get; init; } = Array.Empty<Header>();
    }
}
