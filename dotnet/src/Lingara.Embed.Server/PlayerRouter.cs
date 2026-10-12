using System;
using System.Diagnostics;
using System.Diagnostics.CodeAnalysis;
using System.Threading;
using System.Threading.Tasks;
using Lingara.Embed.Server.Webhooks;

namespace Lingara.Embed.Server;

/// <summary>A verified event and the player it concerns, or none for a subject the directory does not know.</summary>
public sealed class RoutedEvent
{
    internal RoutedEvent(WebhookEvent webhookEvent, string? playerRef)
    {
        Event = webhookEvent;
        PlayerRef = playerRef;
    }

    public WebhookEvent Event { get; }

    /// <summary>Your player, or null: your own subject, or a player you deleted.</summary>
    public string? PlayerRef { get; }

    public override string ToString() => $"RoutedEvent {{ Event = {Event}, PlayerRef = {PlayerRef} }}";
}

/// <summary>
/// Routes Lingara's webhooks to your players (ADR 1.10.26af D8): the
/// signature on the raw body first, then the directory, so a forged event
/// never reaches your store. <see cref="HandleAsync"/> answers with the
/// status your route sends, the same in all eight kits.
/// </summary>
public sealed class PlayerRouter
{
    private readonly IPlayerDirectory directory;
    private readonly WebhookVerifier verifier;

    /// <summary>A router over your webhook secrets: one, or two during a rotation.</summary>
    /// <exception cref="ArgumentException">A secret is malformed: the misconfiguration surfaces at startup.</exception>
    public PlayerRouter(IPlayerDirectory directory, params string[] secrets)
        : this(directory, new WebhookVerifier(secrets))
    {
    }

    /// <summary>A router over a verifier you built (a clock of your own, for tests).</summary>
    public PlayerRouter(IPlayerDirectory directory, WebhookVerifier verifier)
    {
        ArgumentNullException.ThrowIfNull(directory);
        ArgumentNullException.ThrowIfNull(verifier);
        this.directory = directory;
        this.verifier = verifier;
    }

    /// <summary>
    /// Where a <c>500</c>'s cause goes: the exception your handler or your
    /// directory threw. Default <see cref="Trace.TraceError(string)"/>.
    /// </summary>
    public Action<Exception> OnError { get; init; } = DefaultOnError;

    /// <summary>Verifies the raw body, then looks its subject up.</summary>
    /// <exception cref="WebhookVerificationException">Before the directory is called.</exception>
    public async ValueTask<RoutedEvent> RouteAsync(ReadOnlyMemory<byte> body, Func<string, string?> header, CancellationToken ct = default)
    {
        var verified = verifier.Verify(body.Span, header);
        var player = await directory.PlayerRefForAsync(verified.Subject, ct).ConfigureAwait(false);
        return new RoutedEvent(verified, player);
    }

    /// <summary>
    /// <see cref="RouteAsync"/>, then <paramref name="onEvent"/>; the status
    /// to send with an empty body: <c>204</c> handled, <c>401</c> a bad
    /// signature, <c>400</c> a malformed payload, <c>500</c> the directory or
    /// the handler threw (the exception goes to <see cref="OnError"/>).
    /// </summary>
    [SuppressMessage("Design", "CA1031", Justification = "Any failure of the integrator's handler or store is a 500 by D8's table.")]
    public async Task<int> HandleAsync(ReadOnlyMemory<byte> body, Func<string, string?> header, Func<RoutedEvent, CancellationToken, Task> onEvent, CancellationToken ct = default)
    {
        ArgumentNullException.ThrowIfNull(onEvent);
        WebhookEvent verified;
        try
        {
            verified = verifier.Verify(body.Span, header);
        }
        catch (WebhookVerificationException e)
        {
            return e.Reason == WebhookVerificationException.MalformedPayload ? 400 : 401;
        }

        try
        {
            var player = await directory.PlayerRefForAsync(verified.Subject, ct).ConfigureAwait(false);
            await onEvent(new RoutedEvent(verified, player), ct).ConfigureAwait(false);
            return 204;
        }
        catch (Exception e) when (!(e is OperationCanceledException && ct.IsCancellationRequested))
        {
            OnError(e);
            return 500;
        }
    }

    public override string ToString() => $"PlayerRouter {{ Verifier = {verifier} }}";

    private static void DefaultOnError(Exception e) =>
        Trace.TraceError("lingara webhook answered 500: {0}: {1}", e.GetType().FullName, e.Message);
}
