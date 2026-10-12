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
    /// K1's Http token source (<c>TokenSourceConfig::Http</c>, D6): the core POSTs
    /// <c>{}</c> to <see cref="Url"/> with <see cref="Headers"/> and reads the handoff body itself.
    /// </summary>
    public sealed class TokenEndpoint
    {
        public string Url { get; init; } = string.Empty;

        public IReadOnlyList<Header> Headers { get; init; } = Array.Empty<Header>();
    }
}
