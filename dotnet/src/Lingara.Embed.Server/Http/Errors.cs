using System;

namespace Lingara.Embed.Server.Http;

/// <summary>
/// Contract K3's family (ADR 1.10.26af D10): one base, so a single
/// <c>catch (LingaraException)</c> handles every refusal from Lingara. A
/// network failure stays .NET's own <see cref="System.Net.Http.HttpRequestException"/>,
/// and the 30 s token exchange deadline a <see cref="TimeoutException"/>.
/// No variant's message carries a secret, a token or a request body.
/// </summary>
public abstract class LingaraException : Exception
{
    private protected LingaraException(int status, string message, TimeSpan? retryAfter)
        : base(message)
    {
        Status = status;
        RetryAfter = retryAfter;
    }

    /// <summary>The HTTP status.</summary>
    public int Status { get; }

    /// <summary>The <c>Retry-After</c> wait, when the answer carried one.</summary>
    public TimeSpan? RetryAfter { get; }
}

/// <summary>
/// A <c>/v1/</c> refusal: the envelope's <c>{code, error}</c>, or
/// <c>http_&lt;status&gt;</c> for a body that is not an envelope (a proxy's
/// HTML 502, an empty 500).
/// </summary>
public sealed class LingaraApiException : LingaraException
{
    internal LingaraApiException(int status, string code, string message, TimeSpan? retryAfter)
        : base(status, message, retryAfter)
    {
        Code = code;
    }

    /// <summary>The server's code, such as <c>embed_needs_metered</c>.</summary>
    public string Code { get; }

    public override string ToString() => $"LingaraApiException {{ Status = {Status}, Code = {Code}, Message = {Message} }}";
}

/// <summary>A refusal from <c>/oauth/token</c>, in RFC 6749's <c>{error, error_description}</c>.</summary>
public sealed class OAuthException : LingaraException
{
    internal OAuthException(int status, string error, string? description, TimeSpan? retryAfter)
        : base(status, $"the token exchange was refused: {error}", retryAfter)
    {
        Error = error;
        Description = description;
    }

    /// <summary>The RFC 6749 error, or <c>http_&lt;status&gt;</c> for a body that is not one.</summary>
    public string Error { get; }

    public string? Description { get; }

    public override string ToString() => $"OAuthException {{ Status = {Status}, Error = {Error} }}";
}

/// <summary>A <c>503</c> whose body is not JSON: Lingara is in maintenance. Never retried.</summary>
public sealed class MaintenanceException : LingaraException
{
    internal MaintenanceException(string body, TimeSpan? retryAfter)
        : base(503, "Lingara is under maintenance", retryAfter)
    {
        Body = body;
    }

    /// <summary>The plain-text body, truncated to 1 KiB.</summary>
    public string Body { get; }

    public override string ToString() => "MaintenanceException { Status = 503 }";
}
