using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Net;
using System.Net.Http;
using System.Text;
using System.Text.Json.Nodes;
using System.Threading;
using System.Threading.Tasks;
using Lingara.Embed.Server.Http;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.Logging;

namespace Lingara.Embed.Server.Conformance;

/// <summary>
/// The ES harness on Lingara.Embed.Server (ADR 1.10.26af D13): every ES case
/// through the kit's public API, a port of the Rust reference harness
/// (<c>core/conformance-es</c>). It binds its webhook receiver to port 0,
/// prints <c>listening &lt;port&gt;</c>, and for each case fetches it, builds
/// a fresh server and directory, arms it, runs the <c>kit:</c> jobs, waits for
/// every delivery, judges what only the caller sees, finishes, and writes one
/// result line. There is no skip.
/// </summary>
internal static class Program
{
    internal const string Lang = "dotnet";

    public static async Task<int> Main()
    {
        var sink = new Sink();
        // The routers keep their default OnError, Trace.TraceError, so ES5's
        // sweep reads D8's default facility through this listener.
        Trace.Listeners.Add(new SinkListener(sink));
        var named = Env("LINGARA_EMBED_WEBHOOK_SECRETS").Split(',');
        var secrets = new Dictionary<string, string>(StringComparer.Ordinal) { ["primary"] = named[0], ["secondary"] = named.ElementAtOrDefault(1) ?? named[0] };
        var receiver = new Receiver(secrets, sink);

        var builder = WebApplication.CreateSlimBuilder();
        builder.Logging.ClearProviders();
        builder.WebHost.UseKestrel(o => o.Listen(IPAddress.Loopback, 0));
        await using var app = builder.Build();
        app.Run(receiver.DeliverAsync);
        await app.StartAsync();
        Console.Out.WriteLine($"listening {new Uri(app.Urls.First()).Port}");
        Console.Out.Flush();

        var server = Env("LINGARA_EMBED_SERVER_URL");
        var cases = Environment.GetEnvironmentVariable("LINGARA_EMBED_CASES") ?? "conformance/es/cases";
        using var control = new HttpClient();
        using (var results = new StreamWriter(Env("LINGARA_EMBED_RESULTS")))
        {
            foreach (var id in CaseIds(cases))
            {
                var run = new CaseRun(control, receiver, server, id);
                results.WriteLine((await run.RunAsync()).ToJsonString());
                results.Flush();
            }
        }

        await app.StopAsync();
        return 0;
    }

    internal static string Env(string name) => Environment.GetEnvironmentVariable(name) ?? throw new InvalidOperationException($"{name} is not set");

    /// <summary><c>&lt;group&gt;/&lt;slug&gt;.yaml</c> → <c>&lt;group&gt;.&lt;slug&gt;</c>, sorted ordinally.</summary>
    private static IEnumerable<string> CaseIds(string dir) =>
        Directory.GetDirectories(dir)
            .SelectMany(g => Directory.GetFiles(g, "*.yaml").Select(f => Path.GetFileName(g) + "." + Path.GetFileNameWithoutExtension(f)))
            .OrderBy(id => id, StringComparer.Ordinal);
}

/// <summary>One case, armed, driven, judged and finished.</summary>
internal sealed class CaseRun
{
    private static readonly string[] SecretPrefixes = ["lgr_et_", "lgr_cs_", "lgr_whsec_"];

    private readonly HttpClient control;
    private readonly Receiver receiver;
    private readonly string server;
    private readonly string id;
    private readonly JsonArray mismatches = new();

    internal CaseRun(HttpClient control, Receiver receiver, string server, string id)
    {
        this.control = control;
        this.receiver = receiver;
        this.server = server;
        this.id = id;
    }

    internal async Task<JsonObject> RunAsync()
    {
        JsonNode kase;
        try
        {
            kase = JsonNode.Parse(await control.GetStringAsync(Control("cases")))!;
        }
        catch (HttpRequestException e)
        {
            mismatches.Add($"cannot read the case: {e.Message}");
            return Result();
        }

        var directory = await Recording.SeededAsync(kase["directory"]?["seed"] as JsonObject);
        var deliver = kase["deliver"] as JsonArray ?? new JsonArray();
        receiver.Arm(new Armed(deliver, directory));
        receiver.Sink.Take();
        using var embed = NewServer(directory);

        await PostAsync("arm", null);
        var outcome = await Jobs.RunAsync(embed, kase["kit"] as JsonArray ?? new JsonArray());
        var seen = await receiver.WaitAsync(deliver.Count, TimeSpan.FromSeconds(15));
        foreach (var m in Jobs.Judge(id, kase["expect"], outcome, directory))
        {
            mismatches.Add(m);
        }

        HandlerMismatches(deliver, seen);
        if (outcome.Render)
        {
            RenderMismatches(embed, directory, outcome);
        }

        await FinishAsync(seen);
        receiver.Disarm();
        return Result();
    }

    /// <summary>A fresh server per case, so no token outlives its case.</summary>
    private EmbedServer NewServer(Recording directory)
    {
        var options = new EmbedServerOptions
        {
            ClientId = Program.Env("LINGARA_EMBED_CLIENT_ID"),
            ClientSecret = Program.Env("LINGARA_EMBED_CLIENT_SECRET"),
            BaseUrl = new Uri(server),
            TokenUrl = new Uri(server + "/oauth/token"),
        };
        return new EmbedServer(options, directory);
    }

    /// <summary><c>{"handlers": [player_ref | null, …]}</c> in delivery order; the server's mismatches join ours.</summary>
    private async Task FinishAsync(Seen?[] seen)
    {
        var handlers = new JsonArray(seen.Select(s => (JsonNode?)(s?.PlayerRef is { } p ? JsonValue.Create(p) : null)).ToArray());
        var verdict = await PostAsync("finish", new JsonObject { ["handlers"] = handlers });
        foreach (var m in verdict?["mismatches"] as JsonArray ?? new JsonArray())
        {
            mismatches.Add(m?.DeepClone());
        }
    }

    private Uri Control(string verb) => new($"{server}/__conformance/{verb}/{id}");

    private JsonObject Result() => new()
    {
        ["case"] = id,
        ["lang"] = Program.Lang,
        ["result"] = mismatches.Count == 0 ? "pass" : "fail",
        ["mismatches"] = mismatches.DeepClone(),
    };

    private async Task<JsonNode?> PostAsync(string verb, JsonNode? body)
    {
        try
        {
            using var content = new StringContent(body?.ToJsonString() ?? string.Empty, Encoding.UTF8, "application/json");
            using var res = await control.PostAsync(Control(verb), content);
            var text = await res.Content.ReadAsStringAsync();
            if (!res.IsSuccessStatusCode)
            {
                mismatches.Add($"cannot {verb}: {(int)res.StatusCode} {text}");
                return null;
            }

            return text.Length == 0 ? null : JsonNode.Parse(text);
        }
        catch (HttpRequestException e)
        {
            mismatches.Add($"cannot {verb}: {e.Message}");
            return null;
        }
    }

    /// <summary>Each delivery's <c>expect.handler</c> against what its <c>onEvent</c> saw.</summary>
    private void HandlerMismatches(JsonArray deliver, Seen?[] seen)
    {
        for (var n = 0; n < deliver.Count; n++)
        {
            var want = deliver[n]?["expect"]?["handler"];
            var got = n < seen.Length ? seen[n] : null;
            if (!HandlerMatches(want, got))
            {
                mismatches.Add($"{id}: fail handler: deliver[{n}] saw {got?.ToString() ?? "nothing"}, expected {want?.ToJsonString() ?? "null"}");
            }
        }
    }

    private static bool HandlerMatches(JsonNode? want, Seen? got)
    {
        if (want is JsonValue v && v.TryGetValue<string>(out var word))
        {
            return word == "not_called" && got is null;
        }

        return got is not null && (want is not JsonObject o || Agrees(o, got));
    }

    /// <summary>Each key the expectation names (<c>player_ref</c>, <c>event_id</c>, <c>unknown</c>) agrees.</summary>
    private static bool Agrees(JsonObject o, Seen got)
    {
        var player = !o.TryGetPropertyValue("player_ref", out var p) || p?.GetValue<string>() == got.PlayerRef;
        var evt = o["event_id"] is not { } e || e.GetValue<string>() == got.EventId;
        var unknown = o["unknown"] is not { } u || u.GetValue<bool>() == got.Unknown;
        return player && evt && unknown;
    }

    /// <summary>ES5: no rendering of a kit object, and no captured log line, carries a secret prefix.</summary>
    private void RenderMismatches(EmbedServer embed, Recording directory, Outcome outcome)
    {
        var router = receiver.Router(new JsonObject { ["router"] = new JsonObject { ["secrets"] = new JsonArray("primary", "secondary") } }, directory);
        var renderings = new List<string> { embed.ToString(), router.ToString(), directory.ToString(), outcome.Minted?.ToString() ?? string.Empty };
        renderings.AddRange(outcome.Raised.Select(e => e.ToString()));
        renderings.AddRange(receiver.Sink.Take());
        var leaks = renderings.Count(r => SecretPrefixes.Any(p => r.Contains(p, StringComparison.Ordinal)));
        if (leaks > 0)
        {
            mismatches.Add($"{id}: fail render: {leaks} rendering(s) carry a secret prefix");
        }
    }
}

/// <summary>What the jobs left behind.</summary>
internal sealed class Outcome
{
    internal MintedToken? Minted { get; set; }

    internal string? Handoff { get; set; }

    internal JsonObject? Error { get; set; }

    internal List<Exception> Raised { get; } = new();

    internal bool Render { get; set; }
}

/// <summary>A case's <c>kit:</c> jobs through <see cref="EmbedServer"/>, and the comparisons only its caller can make.</summary>
internal static class Jobs
{
    private static readonly string[] Keys = ["minted", "handoff", "error", "directory", "directory_lookups"];

    /// <summary>Every job in order; a raised error is recorded and the next job runs.</summary>
    internal static async Task<Outcome> RunAsync(EmbedServer server, JsonArray jobs)
    {
        var outcome = new Outcome();
        foreach (var job in jobs.OfType<JsonObject>())
        {
            try
            {
                await RunOneAsync(server, job, outcome);
            }
            catch (Exception e)
            {
                outcome.Raised.Add(e);
                outcome.Error = Raised(e);
            }
        }

        return outcome;
    }

    private static async Task RunOneAsync(EmbedServer server, JsonObject job, Outcome outcome)
    {
        if (job["mint"] is JsonObject mint)
        {
            var scopes = (mint["scopes"] as JsonArray)?.Select(s => s!.GetValue<string>()).ToList();
            var options = new MintOptions { Scopes = scopes, Origin = mint["origin"]?.GetValue<string>() };
            outcome.Minted = await server.MintTokenAsync(mint["player_ref"]!.GetValue<string>(), options);
        }
        else if (job["delete"] is JsonObject delete)
        {
            await server.DeletePlayerAsync(delete["player_ref"]!.GetValue<string>());
        }
        else if (job.ContainsKey("handoff"))
        {
            outcome.Handoff = outcome.Minted is null ? null : TokenHandoff.Body(outcome.Minted);
        }
        else
        {
            outcome.Render = true;
        }
    }

    /// <summary><c>{status, code}</c>: an API error's code, an OAuth error's <c>Error</c>, or <c>maintenance</c>.</summary>
    private static JsonObject Raised(Exception e) => e switch
    {
        LingaraApiException a => new JsonObject { ["status"] = a.Status, ["code"] = a.Code },
        OAuthException o => new JsonObject { ["status"] = o.Status, ["code"] = o.Error },
        MaintenanceException => new JsonObject { ["status"] = 503, ["code"] = "maintenance" },
        _ => new JsonObject { ["status"] = 0, ["code"] = e.GetType().Name },
    };

    /// <summary>The six fields in wire form, the token read through its accessor.</summary>
    private static JsonObject Wire(MintedToken t) => new()
    {
        ["token"] = t.ExposeToken(),
        ["expires_at"] = t.ExpiresAtText,
        ["expires_in"] = (long)t.ExpiresIn.TotalSeconds,
        ["subject"] = t.Subject,
        ["scopes"] = new JsonArray(t.Scopes.Select(s => (JsonNode?)JsonValue.Create(s)).ToArray()),
        ["account_linked"] = t.AccountLinked,
    };

    /// <summary>Each of the five keys the case names, as a <c>fail &lt;key&gt;</c> line when it differs.</summary>
    internal static IEnumerable<string> Judge(string id, JsonNode? expect, Outcome outcome, Recording directory)
    {
        foreach (var key in Keys)
        {
            JsonNode? have = key switch
            {
                "minted" => outcome.Minted is null ? null : Wire(outcome.Minted),
                "handoff" => outcome.Handoff is null ? null : JsonValue.Create(outcome.Handoff),
                "error" => outcome.Error,
                "directory" => new JsonObject(directory.Snapshot().Select(r => KeyValuePair.Create(r.Key, (JsonNode?)JsonValue.Create(r.Value)))),
                _ => JsonValue.Create(directory.Lookups),
            };
            var named = expect is JsonObject o && o.ContainsKey(key);
            var want = named ? expect![key] : null;
            // A key the case names must match; an error it did not expect is a mismatch too.
            var differs = named ? Canon(have) != Canon(want) : key == "error" && have is not null;
            if (differs)
            {
                yield return $"{id}: fail {key}: got {Canon(have)}, expected {Canon(want)}";
            }
        }
    }

    /// <summary>A canonical text: object keys sorted, null members dropped, at any depth.</summary>
    internal static string Canon(JsonNode? node) => node switch
    {
        null => "nothing",
        JsonObject o => "{" + string.Join(",", o.Where(p => p.Value is not null).OrderBy(p => p.Key, StringComparer.Ordinal).Select(p => JsonValue.Create(p.Key).ToJsonString() + ":" + Canon(p.Value))) + "}",
        JsonArray a => "[" + string.Join(",", a.Select(Canon)) + "]",
        _ => node.ToJsonString(),
    };
}

/// <summary>What one delivery's <c>onEvent</c> saw.</summary>
internal sealed record Seen(string? PlayerRef, string EventId, bool Unknown);

/// <summary>The armed case, as the receiver needs it.</summary>
internal sealed class Armed
{
    internal Armed(JsonArray deliver, Recording directory)
    {
        Deliver = deliver;
        Directory = directory;
        Seen = new Seen?[deliver.Count];
    }

    internal JsonArray Deliver { get; }

    internal Recording Directory { get; }

    internal Seen?[] Seen { get; }

    internal int Received { get; set; }

    internal int Answered { get; set; }
}

/// <summary>
/// The webhook receiver the conformance server posts to. Delivery <c>n</c> of
/// the armed case is answered through a <see cref="PlayerRouter"/> built from
/// that item's <c>router.secrets</c>, whose <c>onEvent</c> records what it saw
/// and raises when the item's <c>handler</c> is <c>raise</c>.
/// </summary>
internal sealed class Receiver
{
    private readonly IReadOnlyDictionary<string, string> secrets;
    private readonly object sync = new();
    private Armed? armed;

    internal Receiver(IReadOnlyDictionary<string, string> secrets, Sink sink)
    {
        this.secrets = secrets;
        Sink = sink;
    }

    internal Sink Sink { get; }

    internal void Arm(Armed? next)
    {
        lock (sync)
        {
            armed = next;
        }
    }

    internal void Disarm() => Arm(null);

    /// <summary>The router delivery <paramref name="item"/> is answered through.</summary>
    internal PlayerRouter Router(JsonNode? item, IPlayerDirectory directory)
    {
        var names = (item?["router"]?["secrets"] as JsonArray)?.Select(n => n!.GetValue<string>()).ToArray() ?? ["primary"];
        return new PlayerRouter(directory, names.Select(n => secrets.TryGetValue(n, out var s) ? s : throw new InvalidOperationException($"no secret named {n}")).ToArray());
    }

    internal async Task DeliverAsync(HttpContext ctx)
    {
        int n;
        Armed? current;
        lock (sync)
        {
            current = armed;
            n = current is null ? -1 : current.Received++;
        }

        if (current is null)
        {
            ctx.Response.StatusCode = StatusCodes.Status409Conflict;
            return;
        }

        using var buffer = new MemoryStream();
        await ctx.Request.Body.CopyToAsync(buffer);
        var item = n < current.Deliver.Count ? current.Deliver[n] : null;
        var raise = item?["handler"]?.GetValue<string>() == "raise";
        int status;
        try
        {
            var router = Router(item, current.Directory);
            status = await router.HandleAsync(buffer.ToArray(), name => ctx.Request.Headers.TryGetValue(name, out var v) ? v.ToString() : null, (routed, _) =>
            {
                lock (sync)
                {
                    current.Seen[n] = new Seen(routed.PlayerRef, routed.Event.Id, routed.Event.IsUnknown);
                }

                return raise ? throw new InvalidOperationException("the conformance handler raised, as the case asked") : Task.CompletedTask;
            });
        }
        catch (Exception e)
        {
            Sink.Add($"harness: {e.Message}");
            status = StatusCodes.Status500InternalServerError;
        }

        lock (sync)
        {
            current.Answered++;
        }

        ctx.Response.StatusCode = status;
    }

    /// <summary>Until every delivery has been answered, or the deadline; then what each saw.</summary>
    internal async Task<Seen?[]> WaitAsync(int count, TimeSpan limit)
    {
        var deadline = DateTime.UtcNow + limit;
        while (true)
        {
            lock (sync)
            {
                if (armed is null || armed.Answered >= count || DateTime.UtcNow > deadline)
                {
                    return armed?.Seen.ToArray() ?? Array.Empty<Seen?>();
                }
            }

            await Task.Delay(20);
        }
    }
}

/// <summary>
/// The case's <see cref="MemoryPlayerDirectory"/>, with its lookups counted:
/// a lookup changes no contents, so only a count shows a forged delivery
/// never reached the store.
/// </summary>
internal sealed class Recording : IPlayerDirectory
{
    private readonly MemoryPlayerDirectory inner = new();
    private int lookups;

    internal int Lookups => Volatile.Read(ref lookups);

    internal static async Task<Recording> SeededAsync(JsonObject? seed)
    {
        var directory = new Recording();
        foreach (var (subject, player) in seed ?? new JsonObject())
        {
            await directory.inner.RememberAsync(player!.GetValue<string>(), subject);
        }

        return directory;
    }

    internal IReadOnlyDictionary<string, string> Snapshot() => inner.Snapshot();

    public ValueTask RememberAsync(string playerRef, string subject, CancellationToken ct = default) => inner.RememberAsync(playerRef, subject, ct);

    public ValueTask<string?> PlayerRefForAsync(string subject, CancellationToken ct = default)
    {
        Interlocked.Increment(ref lookups);
        return inner.PlayerRefForAsync(subject, ct);
    }

    public ValueTask ForgetAsync(string playerRef, CancellationToken ct = default) => inner.ForgetAsync(playerRef, ct);

    public override string ToString() => $"Recording {{ Inner = {inner}, Lookups = {Lookups} }}";
}

/// <summary>Every line the routers' <c>OnError</c> and the harness wrote.</summary>
internal sealed class Sink
{
    private readonly ConcurrentQueue<string> lines = new();

    internal void Add(string line) => lines.Enqueue(line);

    internal List<string> Take()
    {
        var taken = new List<string>();
        while (lines.TryDequeue(out var line))
        {
            taken.Add(line);
        }

        return taken;
    }
}

/// <summary><see cref="Trace"/> into the sink: the router's default <c>OnError</c> facility.</summary>
internal sealed class SinkListener : TraceListener
{
    private readonly Sink sink;

    internal SinkListener(Sink sink) => this.sink = sink;

    public override void Write(string? message) => sink.Add(message ?? string.Empty);

    public override void WriteLine(string? message) => sink.Add(message ?? string.Empty);
}
