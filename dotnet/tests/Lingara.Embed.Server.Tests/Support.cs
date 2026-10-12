using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Text.Json;
using Lingara.Embed.Server.Webhooks;

namespace Lingara.Embed.Server.Tests;

/// <summary>The repository root and its shared vectors, read as every server kit reads them.</summary>
internal static class Repo
{
    internal static readonly string Root = typeof(Repo).Assembly.GetCustomAttributes<AssemblyMetadataAttribute>().Single(a => a.Key == "RepoRoot").Value!;

    internal static string Path(params string[] parts) => System.IO.Path.Combine(new[] { Root }.Concat(parts).ToArray());

    internal static JsonElement Vectors(string name)
    {
        using var doc = JsonDocument.Parse(File.ReadAllText(Path("conformance", "vectors", name)));
        return doc.RootElement.Clone();
    }

    internal static string Version => File.ReadAllText(Path("VERSION")).Trim();
}

/// <summary>A clock that stands still until a test moves it.</summary>
internal sealed class FixedTime : TimeProvider
{
    internal FixedTime(DateTimeOffset now) => Now = now;

    internal DateTimeOffset Now { get; set; }

    public override DateTimeOffset GetUtcNow() => Now;
}

/// <summary>One <c>webhook-signatures.json</c> vector, ready to verify at its own <c>now</c>.</summary>
internal sealed class SignatureVector
{
    private readonly Dictionary<string, string> headers;

    private SignatureVector(JsonElement v)
    {
        Name = v.GetProperty("name").GetString()!;
        Secrets = v.GetProperty("secrets").EnumerateArray().Select(s => s.GetString()!).ToArray();
        headers = v.GetProperty("headers").EnumerateObject().ToDictionary(p => p.Name, p => p.Value.GetString()!, StringComparer.OrdinalIgnoreCase);
        Body = System.Text.Encoding.UTF8.GetBytes(v.GetProperty("body").GetString()!);
        Now = DateTimeOffset.FromUnixTimeSeconds(v.GetProperty("now").GetInt64());
        Expect = v.GetProperty("expect").Clone();
    }

    internal string Name { get; }

    internal string[] Secrets { get; }

    internal byte[] Body { get; }

    internal DateTimeOffset Now { get; }

    internal JsonElement Expect { get; }

    /// <summary>A case-insensitive lookup, as ASP.NET Core's headers are.</summary>
    internal string? Header(string name) => headers.TryGetValue(name, out var v) ? v : null;

    internal WebhookVerifier Verifier() => new(Secrets, new FixedTime(Now));

    internal static IReadOnlyList<SignatureVector> All() =>
        Repo.Vectors("webhook-signatures.json").GetProperty("vectors").EnumerateArray().Select(v => new SignatureVector(v)).ToList();

    internal static SignatureVector Named(string name) => All().Single(v => v.Name == name);
}
