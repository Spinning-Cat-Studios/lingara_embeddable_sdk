// The token route, the delete and the widget page of an ASP.NET Core server
// over Lingara.Embed.Server (ADR 1.10.26af D13). Compiled by
// `make test-dotnet-server` (dotnet/tests/Lingara.Embed.Server.Snippets).
using System;
using System.Data.Common;
using System.Security.Claims;
using System.Threading;
using Lingara.Embed.Server;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Http;

public static class LingaraRoutes
{
    public static void MapLingara(this WebApplication app, DbDataSource db)
    {
        // lingara:begin mint
        // One EmbedServer for the process: it caches the client-credentials token.
        var lingara = new EmbedServer(new EmbedServerOptions
        {
            ClientId = Environment.GetEnvironmentVariable("LINGARA_CLIENT_ID")!,         // lgr_cid_…
            ClientSecret = Environment.GetEnvironmentVariable("LINGARA_CLIENT_SECRET")!, // lgr_cs_…, from your secret store
        }, new SqlPlayerDirectory(db));

        // Your own session says who the player is. Rate-limit this route per session.
        app.MapPost("/lingara/token", async (ClaimsPrincipal user, CancellationToken ct) =>
        {
            var playerRef = user.FindFirstValue(ClaimTypes.NameIdentifier);
            if (playerRef is null)
            {
                return Results.Unauthorized();
            }

            // This page's origin, one of your client's embed origins. A game's server omits it.
            var minted = await lingara.MintTokenAsync(playerRef, new MintOptions { Origin = "https://game.example" }, ct);
            return Results.Text(TokenHandoff.Body(minted), "application/json"); // {"token":"lgr_et_…","expires_at":"…"}
        }).RequireAuthorization().RequireRateLimiting("lingara-token");
        // lingara:end
        // lingara:begin deletePlayer
        // From your own account-deletion path: revokes the player's tokens at
        // Lingara, then forgets their subjects. An unknown player is a success.
        app.MapDelete("/account", async (ClaimsPrincipal user, CancellationToken ct) =>
        {
            var playerRef = user.FindFirstValue(ClaimTypes.NameIdentifier)!;
            await lingara.DeletePlayerAsync(playerRef, ct);
            // …then delete your own rows for this player.
            return Results.NoContent();
        }).RequireAuthorization();
        // lingara:end
        // lingara:begin widgetSnippet
        app.MapGet("/practice/{planId}", (string planId, HttpContext http) =>
        {
            var widget = WidgetSnippet.Render(new SnippetOptions
            {
                PlanId = planId,                              // the plan your server made for this player
                TokenEndpoint = "/lingara/token",             // same origin: the element sends this page's cookies
                Nonce = http.Items["csp-nonce"] as string,     // your CSP's nonce, if script-src has one
            });
            return Results.Content($"<!doctype html><title>Practice</title>\n{widget}", "text/html");
        }).RequireAuthorization();
        // lingara:end
    }
}
