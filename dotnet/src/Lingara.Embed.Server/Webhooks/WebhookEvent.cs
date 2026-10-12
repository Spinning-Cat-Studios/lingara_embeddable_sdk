using System.Text.Json;

namespace Lingara.Embed.Server.Webhooks;

/// <summary>
/// A verified event (ADR 1.10.26af D10): the envelope, and <see cref="Data"/>
/// as raw JSON, since C# has no generated event models; typed arms are a
/// follow-up. A type this kit does not know is <see cref="IsUnknown"/>, never
/// an error: answer it <c>2xx</c> like any other, or Lingara keeps retrying it.
/// Deduplicate on <see cref="Id"/>: delivery is at least once.
/// </summary>
public sealed class WebhookEvent
{
    internal WebhookEvent(string id, string type, string createdAt, string apiVersion, string subject, JsonElement data, bool isUnknown)
    {
        Id = id;
        Type = type;
        CreatedAt = createdAt;
        ApiVersion = apiVersion;
        Subject = subject;
        Data = data;
        IsUnknown = isUnknown;
    }

    /// <summary><c>lgr_evt_…</c>, the deduplication key.</summary>
    public string Id { get; }

    /// <summary>The event type, such as <c>practice.completed</c>.</summary>
    public string Type { get; }

    /// <summary><c>created_at</c>, as the server wrote it (RFC 3339).</summary>
    public string CreatedAt { get; }

    /// <summary>The API version <see cref="Data"/> is rendered at.</summary>
    public string ApiVersion { get; }

    /// <summary>The pairwise <c>lgr_sub_</c> the event concerns.</summary>
    public string Subject { get; }

    /// <summary>The event's <c>data</c>, a JSON object for every known type.</summary>
    public JsonElement Data { get; }

    /// <summary>True for a type newer than this kit (the catalogue is additive).</summary>
    public bool IsUnknown { get; }

    public override string ToString() => $"WebhookEvent {{ Id = {Id}, Type = {Type}, Subject = {Subject} }}";
}
