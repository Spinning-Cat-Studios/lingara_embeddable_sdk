using System;
using System.Collections.Generic;
using System.Linq;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;

namespace Lingara.Embed.Server.Webhooks;

/// <summary>
/// Lingara's Standard Webhooks verifier, written natively (ADR 1.10.26af D10;
/// clients E7 D4, steps 1–6), and held to the libraries'
/// <c>webhook-signatures.json</c>. It stores nothing: deduplicate on
/// <see cref="WebhookEvent.Id"/> yourself, since delivery is at least once.
/// </summary>
public sealed class WebhookVerifier
{
    private const string Prefix = "lgr_whsec_";
    private const int MinKeyBytes = 24;
    private const long ToleranceSeconds = 300;

    private static readonly string[] Envelope = ["id", "type", "created_at", "api_version", "subject"];

    private readonly byte[][] keys;
    private readonly TimeProvider time;

    /// <summary>One secret, or two during a rotation, each <c>lgr_whsec_</c> + padded base64.</summary>
    public WebhookVerifier(params string[] secrets)
        : this(secrets, null)
    {
    }

    /// <param name="secrets">Each <c>lgr_whsec_</c> + standard padded base64 of at least 24 bytes.</param>
    /// <param name="time">The clock the 300 s tolerance is read against, a testing seam; default the system's.</param>
    /// <exception cref="ArgumentException">A secret is not a Lingara webhook secret (the message never holds it).</exception>
    public WebhookVerifier(IEnumerable<string> secrets, TimeProvider? time)
    {
        ArgumentNullException.ThrowIfNull(secrets);
        keys = secrets.Select(Key).ToArray();
        if (keys.Length == 0)
        {
            throw new ArgumentException("a webhook verifier needs at least one secret", nameof(secrets));
        }

        this.time = time ?? TimeProvider.System;
    }

    /// <summary>
    /// Checks the signature over the raw body, then parses it. Pass the body
    /// exactly as received, never a re-serialised object, and a header lookup
    /// that ignores case, as ASP.NET Core's <c>name =&gt; Request.Headers[name]</c> does.
    /// </summary>
    /// <exception cref="WebhookVerificationException">With one of the six reasons.</exception>
    public WebhookEvent Verify(ReadOnlySpan<byte> body, Func<string, string?> header)
    {
        var id = Signed(body, header);
        return Parse(body, id);
    }

    /// <summary>Steps 1–5 alone: the signature, never the body's shape, so never <c>malformed_payload</c>.</summary>
    /// <exception cref="WebhookVerificationException">With one of the five signature reasons.</exception>
    public void VerifySignature(ReadOnlySpan<byte> body, Func<string, string?> header) => Signed(body, header);

    public override string ToString() => $"WebhookVerifier {{ Secrets = [REDACTED] x{keys.Length} }}";

    /// <summary>Step 1: <c>lgr_whsec_</c>, then <c>^[A-Za-z0-9+/]+={0,2}$</c> in whole quanta, decoding to ≥ 24 bytes.</summary>
    private static byte[] Key(string secret)
    {
        var refused = new ArgumentException("a webhook secret must be lgr_whsec_ followed by padded base64 of at least 24 bytes", nameof(secret));
        if (secret is null || !secret.StartsWith(Prefix, StringComparison.Ordinal))
        {
            throw refused;
        }

        var rest = secret[Prefix.Length..];
        var buffer = new byte[rest.Length];
        if (!IsStrictBase64(rest) || !Convert.TryFromBase64String(rest, buffer, out var written) || written < MinKeyBytes)
        {
            throw refused;
        }

        return buffer[..written];
    }

    /// <summary>Before any decoder's leniency: the alphabet, at most two <c>=</c> at the end, whole quanta.</summary>
    private static bool IsStrictBase64(string text)
    {
        var body = text.TrimEnd('=');
        return text.Length % 4 == 0 && text.Length - body.Length <= 2 && body.Length > 0
            && body.All(c => char.IsAsciiLetterOrDigit(c) || c == '+' || c == '/');
    }

    /// <summary>Steps 2–5; the <c>webhook-id</c> on success.</summary>
    private string Signed(ReadOnlySpan<byte> body, Func<string, string?> header)
    {
        ArgumentNullException.ThrowIfNull(header);
        var id = header("webhook-id");
        var stamp = header("webhook-timestamp");
        var signature = header("webhook-signature");
        if (id is null || stamp is null || signature is null)
        {
            throw new WebhookVerificationException(WebhookVerificationException.MissingHeader);
        }

        Tolerate(stamp);
        var content = Content(id, stamp, body);
        var offered = Offered(signature).ToList();
        foreach (var key in keys)
        {
            var expected = HMACSHA256.HashData(key, content);
            if (offered.Any(sig => CryptographicOperations.FixedTimeEquals(expected, sig)))
            {
                return id;
            }
        }

        throw new WebhookVerificationException(WebhookVerificationException.NoMatchingSignature);
    }

    /// <summary>Steps 2–3: one or more ASCII digits, within 300 s of now either way.</summary>
    private void Tolerate(string stamp)
    {
        if (stamp.Length == 0 || !stamp.All(char.IsAsciiDigit) || !long.TryParse(stamp, out var at))
        {
            throw new WebhookVerificationException(WebhookVerificationException.MalformedHeader);
        }

        var now = time.GetUtcNow().ToUnixTimeSeconds();
        if (now - at > ToleranceSeconds)
        {
            throw new WebhookVerificationException(WebhookVerificationException.TimestampTooOld);
        }

        if (at - now > ToleranceSeconds)
        {
            throw new WebhookVerificationException(WebhookVerificationException.TimestampTooNew);
        }
    }

    /// <summary>Step 4: <c>id.timestamp.</c> then the body's bytes, exactly as received.</summary>
    private static byte[] Content(string id, string stamp, ReadOnlySpan<byte> body)
    {
        var head = Encoding.UTF8.GetBytes(id + "." + stamp + ".");
        var content = new byte[head.Length + body.Length];
        head.CopyTo(content, 0);
        body.CopyTo(content.AsSpan(head.Length));
        return content;
    }

    /// <summary>Step 5's candidates: each space-separated <c>v1,</c> element that decodes. Others are skipped.</summary>
    private static IEnumerable<byte[]> Offered(string signature)
    {
        foreach (var element in signature.Split(' '))
        {
            if (!element.StartsWith("v1,", StringComparison.Ordinal))
            {
                continue;
            }

            var encoded = element[3..];
            var buffer = new byte[encoded.Length];
            if (Convert.TryFromBase64String(encoded, buffer, out var written))
            {
                yield return buffer[..written];
            }
        }
    }

    /// <summary>
    /// Step 6, only after a match: an envelope whose <c>id</c> is the
    /// <c>webhook-id</c>, and a known type's <c>data</c> in its shape.
    /// </summary>
    private static WebhookEvent Parse(ReadOnlySpan<byte> body, string id)
    {
        try
        {
            using var doc = JsonDocument.Parse(body.ToArray());
            var root = doc.RootElement;
            var fields = Envelope.Select(k => root.GetProperty(k).GetString()!).ToArray();
            var data = root.GetProperty("data");
            if (fields[0] == id && EventShapes.Conforms(fields[1], data))
            {
                return new WebhookEvent(fields[0], fields[1], fields[2], fields[3], fields[4], data.Clone(), !EventShapes.IsKnown(fields[1]));
            }
        }
        catch (Exception e) when (e is JsonException or KeyNotFoundException or InvalidOperationException)
        {
            // Not JSON, not an object, a field missing or not a string.
        }

        throw new WebhookVerificationException(WebhookVerificationException.MalformedPayload);
    }
}

/// <summary>
/// Why <see cref="WebhookVerifier"/> refused a delivery: one of E7's six
/// reasons. It sits outside <see cref="Http.LingaraException"/>, since no
/// Lingara server answered anything, and its message never carries a
/// secret, a signature or the body.
/// </summary>
public sealed class WebhookVerificationException : Exception
{
    public const string MissingHeader = "missing_header";
    public const string MalformedHeader = "malformed_header";
    public const string TimestampTooOld = "timestamp_too_old";
    public const string TimestampTooNew = "timestamp_too_new";
    public const string NoMatchingSignature = "no_matching_signature";
    public const string MalformedPayload = "malformed_payload";

    internal WebhookVerificationException(string reason)
        : base("webhook verification failed: " + reason)
    {
        Reason = reason;
    }

    /// <summary>One of the six reason strings above.</summary>
    public string Reason { get; }
}
