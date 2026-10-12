using System;
using System.Globalization;

namespace Lingara.Embed
{
    // K1's ids share one counter from 1 to at most i64::MAX, so a long holds every
    // one losslessly and Godot's Variant can carry it (ADR 1.10.26aa D8).

    /// <summary>The id a command's answering event carries in its <c>request</c>.</summary>
    public readonly struct RequestId : IEquatable<RequestId>
    {
        public RequestId(long value) => Value = value;

        public long Value { get; }

        public static bool operator ==(RequestId left, RequestId right) => left.Equals(right);

        public static bool operator !=(RequestId left, RequestId right) => !left.Equals(right);

        public bool Equals(RequestId other) => Value == other.Value;

        public override bool Equals(object? obj) => obj is RequestId other && Equals(other);

        public override int GetHashCode() => Value.GetHashCode();

        public override string ToString() => Value.ToString(CultureInfo.InvariantCulture);
    }

    /// <summary>A practice session's id.</summary>
    public readonly struct SessionId : IEquatable<SessionId>
    {
        public SessionId(long value) => Value = value;

        public long Value { get; }

        public static bool operator ==(SessionId left, SessionId right) => left.Equals(right);

        public static bool operator !=(SessionId left, SessionId right) => !left.Equals(right);

        public bool Equals(SessionId other) => Value == other.Value;

        public override bool Equals(object? obj) => obj is SessionId other && Equals(other);

        public override int GetHashCode() => Value.GetHashCode();

        public override string ToString() => Value.ToString(CultureInfo.InvariantCulture);
    }

    /// <summary>An NPC dialogue's id.</summary>
    public readonly struct DialogueId : IEquatable<DialogueId>
    {
        public DialogueId(long value) => Value = value;

        public long Value { get; }

        public static bool operator ==(DialogueId left, DialogueId right) => left.Equals(right);

        public static bool operator !=(DialogueId left, DialogueId right) => !left.Equals(right);

        public bool Equals(DialogueId other) => Value == other.Value;

        public override bool Equals(object? obj) => obj is DialogueId other && Equals(other);

        public override int GetHashCode() => Value.GetHashCode();

        public override string ToString() => Value.ToString(CultureInfo.InvariantCulture);
    }
}
