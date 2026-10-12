// The webhook route of an ASP.NET Core server over Lingara.Embed.Server, with
// a database-backed player directory beside it (ADR 1.10.26af D13). Compiled
// by `make test-dotnet-server` (dotnet/tests/Lingara.Embed.Server.Snippets).
using System;
using System.Data.Common;
using System.IO;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;
using Lingara.Embed.Server;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Http;

// lingara:begin webhook
public static partial class LingaraWebhook
{
    public static void MapLingaraWebhook(this WebApplication app, DbDataSource db)
    {
        // lgr_whsec_… from your secret store; pass both during a rotation.
        var router = new PlayerRouter(new SqlPlayerDirectory(db), Environment.GetEnvironmentVariable("LINGARA_WEBHOOK_SECRET")!);

        app.MapPost("/lingara/webhook", async (HttpRequest request, CancellationToken ct) =>
        {
            using var body = new MemoryStream();
            await request.Body.CopyToAsync(body, ct); // the raw bytes: never a re-serialised object
            var status = await router.HandleAsync(body.ToArray(), name => request.Headers[name].FirstOrDefault(), async (routed, token) =>
            {
                // Delivery is at least once: skip an Event.Id you have seen. Enqueue
                // slow work and return, since Lingara expects a 2xx within 10 s.
                if (routed.PlayerRef is { } player && routed.Event.Type == "practice.completed")
                {
                    await SaveProgressAsync(db, player, routed.Event.Id, routed.Event.Data.GetRawText(), token);
                }
            }, ct);
            return Results.StatusCode(status); // 204, 400, 401 or 500, always with an empty body
        });
    }
}

/// <summary>
/// Your player directory over your own database (PostgreSQL here):
/// <c>CREATE TABLE lingara_players (player_ref text NOT NULL, subject text PRIMARY KEY)</c>
/// with <c>CREATE INDEX ON lingara_players (player_ref)</c>. Unique on
/// <c>subject</c> only: a player who links an account owns two subjects.
/// </summary>
public sealed class SqlPlayerDirectory(DbDataSource db) : IPlayerDirectory
{
    public async ValueTask RememberAsync(string playerRef, string subject, CancellationToken ct = default)
    {
        await using var cmd = Command(db, "INSERT INTO lingara_players (player_ref, subject) VALUES ($1, $2) ON CONFLICT (subject) DO UPDATE SET player_ref = $1", playerRef, subject);
        await cmd.ExecuteNonQueryAsync(ct);
    }

    public async ValueTask<string?> PlayerRefForAsync(string subject, CancellationToken ct = default)
    {
        await using var cmd = Command(db, "SELECT player_ref FROM lingara_players WHERE subject = $1", subject);
        return await cmd.ExecuteScalarAsync(ct) as string;
    }

    public async ValueTask ForgetAsync(string playerRef, CancellationToken ct = default)
    {
        await using var cmd = Command(db, "DELETE FROM lingara_players WHERE player_ref = $1", playerRef);
        await cmd.ExecuteNonQueryAsync(ct);
    }

    internal static DbCommand Command(DbDataSource db, string sql, params string[] values)
    {
        var cmd = db.CreateCommand(sql);
        foreach (var value in values)
        {
            var p = cmd.CreateParameter();
            p.Value = value;
            cmd.Parameters.Add(p);
        }

        return cmd;
    }
}
// lingara:end
public static partial class LingaraWebhook
{
    private static async Task SaveProgressAsync(DbDataSource db, string player, string eventId, string data, CancellationToken ct)
    {
        await using var cmd = SqlPlayerDirectory.Command(db, "INSERT INTO practice_log (event_id, player_ref, data) VALUES ($1, $2, $3) ON CONFLICT (event_id) DO NOTHING", eventId, player, data);
        await cmd.ExecuteNonQueryAsync(ct);
    }
}
