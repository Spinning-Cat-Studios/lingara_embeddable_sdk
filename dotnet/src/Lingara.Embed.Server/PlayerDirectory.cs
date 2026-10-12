using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;

namespace Lingara.Embed.Server;

/// <summary>
/// Your store of <c>player_ref</c> ↔ <c>subject</c> (ADR 1.10.26af D5): the
/// one mapping a kit cannot derive, since a subject is a pairwise HMAC
/// computed by Lingara. Keep it unique on <c>subject</c> only: a player who
/// links an account owns a second subject beside the guest's. Implement it
/// over your own database, beside your player rows. Whatever it throws, the
/// kit propagates unchanged.
/// </summary>
public interface IPlayerDirectory
{
    /// <summary>Upserts the row keyed on <paramref name="subject"/>. Remembering it again changes nothing.</summary>
    ValueTask RememberAsync(string playerRef, string subject, CancellationToken ct = default);

    /// <summary>The player a subject belongs to, or null for one never seen (your own subject, a deleted player).</summary>
    ValueTask<string?> PlayerRefForAsync(string subject, CancellationToken ct = default);

    /// <summary>Removes every row for the player. Idempotent.</summary>
    ValueTask ForgetAsync(string playerRef, CancellationToken ct = default);
}

/// <summary>
/// A process-local directory, for tests and single-process demos. It is lost
/// on restart and not shared between instances: production code implements
/// <see cref="IPlayerDirectory"/> over its own database.
/// </summary>
public sealed class MemoryPlayerDirectory : IPlayerDirectory
{
    private readonly Dictionary<string, string> rows = new(StringComparer.Ordinal);
    private readonly object sync = new();

    public ValueTask RememberAsync(string playerRef, string subject, CancellationToken ct = default)
    {
        lock (sync)
        {
            rows[subject] = playerRef;
        }

        return ValueTask.CompletedTask;
    }

    public ValueTask<string?> PlayerRefForAsync(string subject, CancellationToken ct = default)
    {
        lock (sync)
        {
            return ValueTask.FromResult(rows.TryGetValue(subject, out var player) ? player : null);
        }
    }

    public ValueTask ForgetAsync(string playerRef, CancellationToken ct = default)
    {
        lock (sync)
        {
            foreach (var subject in rows.Where(r => r.Value == playerRef).Select(r => r.Key).ToList())
            {
                rows.Remove(subject);
            }
        }

        return ValueTask.CompletedTask;
    }

    /// <summary>Every row, subject → player_ref, ordered by subject.</summary>
    public IReadOnlyDictionary<string, string> Snapshot()
    {
        lock (sync)
        {
            return new SortedDictionary<string, string>(rows, StringComparer.Ordinal);
        }
    }

    public override string ToString()
    {
        lock (sync)
        {
            return $"MemoryPlayerDirectory {{ Rows = {rows.Count} }}";
        }
    }
}
