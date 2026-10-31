using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Net.Http;
using System.Text;
using System.Text.Json.Nodes;
using System.Threading;
using Lingara.Embed;
using Lingara.Embed.Events;

namespace Lingara.Embed.Conformance
{
    /// <summary>
    /// Every EK case's <c>host:</c> script through the public C# API (ADR 1.10.26aa
    /// D12, conformance/ek/CONTRACT.md). A <c>command</c> step goes through a
    /// table from each command <c>type</c> to its typed method, and a type with no
    /// row fails the case: there is no skip. Events are matched on their
    /// <c>RawJson</c>. This is an ABI harness, so <c>close</c> disposes, and no
    /// <c>closed</c> event is expected after it.
    /// </summary>
    internal static class Harness
    {
        private const string Lang = "dotnet";

        public static int Main()
        {
            var server = Environment.GetEnvironmentVariable("LINGARA_EMBED_SERVER_URL") ?? throw new InvalidOperationException("LINGARA_EMBED_SERVER_URL");
            var results = Environment.GetEnvironmentVariable("LINGARA_EMBED_RESULTS") ?? throw new InvalidOperationException("LINGARA_EMBED_RESULTS");
            var cases = Environment.GetEnvironmentVariable("LINGARA_EMBED_CASES") ?? "conformance/ek/cases";
            using var http = new HttpClient();
            using var output = new StreamWriter(results);
            foreach (var id in CaseIds(cases))
            {
                output.WriteLine(RunCase(http, server, id).ToJsonString());
            }

            return 0;
        }

        /// <summary><c>&lt;group&gt;/&lt;slug&gt;.yaml</c> → <c>&lt;group&gt;.&lt;slug&gt;</c>, sorted ordinally.</summary>
        private static IEnumerable<string> CaseIds(string dir) =>
            Directory.GetDirectories(dir)
                .SelectMany(g => Directory.GetFiles(g, "*.yaml").Select(f => Path.GetFileName(g) + "." + Path.GetFileNameWithoutExtension(f)))
                .OrderBy(id => id, StringComparer.Ordinal);

        private static JsonObject RunCase(HttpClient http, string server, string id)
        {
            string Control(string verb) => $"{server}/__conformance/{verb}/{id}";
            var mismatches = new JsonArray();
            JsonNode? kase;
            try
            {
                kase = JsonNode.Parse(http.GetStringAsync(new Uri(Control("cases"))).GetAwaiter().GetResult());
            }
            catch (HttpRequestException e)
            {
                return Result(id, new JsonArray($"cannot read the case: {e.Message}"));
            }

            Post(http, Control("arm"), null, mismatches);
            var run = new HostRun(server);
            run.Run(kase!, id);
            foreach (var problem in run.Problems)
            {
                mismatches.Add(problem);
            }

            if (kase!["expect"]?["events"] is JsonArray want)
            {
                var got = Matching.WithoutNulls(new JsonArray(run.Matched.Select(m => m.DeepClone()).ToArray()));
                if (!JsonNode.DeepEquals(got, Matching.WithoutNulls(want)))
                {
                    mismatches.Add($"{id}: the host saw {got.ToJsonString()}, expected {want.ToJsonString()}");
                }
            }

            var verdict = Post(http, Control("finish"), "{}", mismatches);
            foreach (var m in (verdict?["mismatches"] as JsonArray) ?? new JsonArray())
            {
                mismatches.Add(m?.DeepClone());
            }

            return Result(id, mismatches);
        }

        private static JsonNode? Post(HttpClient http, string url, string? body, JsonArray mismatches)
        {
            try
            {
                using var content = new StringContent(body ?? string.Empty, Encoding.UTF8, "application/json");
                using var response = http.PostAsync(new Uri(url), content).GetAwaiter().GetResult();
                response.EnsureSuccessStatusCode();
                var text = response.Content.ReadAsStringAsync().GetAwaiter().GetResult();
                return string.IsNullOrWhiteSpace(text) ? null : JsonNode.Parse(text);
            }
            catch (HttpRequestException e)
            {
                mismatches.Add($"cannot reach {url}: {e.Message}");
                return null;
            }
        }

        private static JsonObject Result(string id, JsonArray mismatches) => new JsonObject
        {
            ["case"] = id,
            ["lang"] = Lang,
            ["result"] = mismatches.Count == 0 ? "pass" : "fail",
            ["mismatches"] = mismatches,
        };
    }

    /// <summary>One case's script against one client.</summary>
    internal sealed class HostRun
    {
        private readonly string server;
        private readonly Queue<JsonNode> local = new Queue<JsonNode>();
        private LingaraEmbedClient? client;
        private long? last;

        public HostRun(string server) => this.server = server;

        public List<string> Problems { get; } = new List<string>();

        public List<JsonNode> Matched { get; } = new List<JsonNode>();

        private LingaraEmbedClient Client => client ?? throw new InvalidOperationException("no start step ran");

        public void Run(JsonNode kase, string id)
        {
            var steps = kase["host"] as JsonArray ?? new JsonArray();
            for (var n = 0; n < steps.Count; n++)
            {
                var step = steps[n]!.AsObject().First();
                try
                {
                    Step(step.Key, step.Value ?? new JsonObject());
                }
                catch (Exception e) when (e is InvalidOperationException || e is FormatException || e is EmbedException)
                {
                    Problems.Add($"{id}: host[{n}].{step.Key}: {e.Message}");
                    break;
                }
            }

            client?.Dispose();
        }

        private void Step(string name, JsonNode body)
        {
            switch (name)
            {
                case "start": Start(body); break;
                case "provide_token": Refusable(() => Provide(body)); break;
                case "command": Refusable(() => Commands.Run(Client, Substitute(body), id => last = id)); break;
                case "poll_until": PollUntil(body); break;
                case "expect_no_event": ExpectNoEvent(body); break;
                default: Pause(name, body); break;
            }
        }

        /// <summary>The two steps that only wait or stop. An ABI harness frees on <c>close</c>.</summary>
        private void Pause(string name, JsonNode body)
        {
            switch (name)
            {
                case "stop_polling": Thread.Sleep((int)(body["for_ms"]?.GetValue<long>() ?? 0)); break;
                case "close": Client.Dispose(); client = null; break;
                default: throw new InvalidOperationException($"unknown step {name}");
            }
        }

        private void Start(JsonNode body)
        {
            client = LingaraEmbedClient.Create(Commands.Options(server, body["config"] as JsonObject ?? new JsonObject()));
            var token = body["initial_token"]?.GetValue<string>();
            if (token != null && token != "none")
            {
                client.ProvideTokenFor(token, 900);
            }
        }

        private void Provide(JsonNode body)
        {
            var token = body["token"]?.GetValue<string>() ?? string.Empty;
            var update = Commands.Update(body["update_available"]);
            if (body["expires_at"] is JsonNode at)
            {
                Client.ProvideToken(token, at.GetValue<string>(), update);
            }
            else
            {
                Client.ProvideTokenFor(token, body["expires_in_secs"]?.GetValue<uint>() ?? throw new InvalidOperationException("no expires_at or expires_in_secs"), update);
            }
        }

        /// <summary>A refusal is an event, seen before anything still queued (CONTRACT.md).</summary>
        private void Refusable(Action action)
        {
            try
            {
                action();
            }
            catch (EmbedException e) when (e.NativeStatus == null)
            {
                local.Enqueue(new JsonObject { ["type"] = "error", ["error"] = Commands.ErrorJson(e.Error) });
            }
        }

        /// <summary>Any value <c>"$last"</c> becomes the id the latest open answered.</summary>
        private JsonObject Substitute(JsonNode body)
        {
            var copy = body.DeepClone().AsObject();
            foreach (var key in copy.Select(p => p.Key).ToList())
            {
                if (copy[key] is JsonValue v && v.TryGetValue<string>(out var s) && s == "$last")
                {
                    copy[key] = last;
                }
            }

            return copy;
        }

        private JsonNode? Next()
        {
            if (local.Count > 0)
            {
                return local.Dequeue();
            }

            return client != null && client.TryPoll(out var evt) ? JsonNode.Parse(evt.RawJson ?? "{}") : null;
        }

        private JsonNode? Find(string type, JsonNode? wanted, long withinMs)
        {
            var clock = Stopwatch.StartNew();
            while (clock.ElapsedMilliseconds < withinMs)
            {
                var seen = Next();
                if (seen == null)
                {
                    Thread.Sleep(5);
                }
                else if (seen["type"]?.GetValue<string>() == type && Matching.Contains(seen, wanted))
                {
                    return seen;
                }
            }

            return null;
        }

        private void PollUntil(JsonNode body)
        {
            var (type, wanted, within) = (body["event"]!.GetValue<string>(), body["match"], body["within_ms"]?.GetValue<long>() ?? 0);
            Matched.Add(Find(type, wanted, within) ?? throw new InvalidOperationException($"no {type} event matching {wanted?.ToJsonString()} within {within} ms"));
        }

        private void ExpectNoEvent(JsonNode body)
        {
            var (type, wanted, period) = (body["event"]!.GetValue<string>(), body["match"], body["for_ms"]?.GetValue<long>() ?? 0);
            var seen = Find(type, wanted, period);
            if (seen != null)
            {
                throw new InvalidOperationException($"an unexpected {type} event: {seen.ToJsonString()}");
            }
        }
    }

    /// <summary>How a <c>poll_until</c> matches (CONTRACT.md): containment, after dropping nulls.</summary>
    internal static class Matching
    {
        public static JsonNode WithoutNulls(JsonNode node) => node switch
        {
            JsonObject o => new JsonObject(o.Where(p => p.Value != null).Select(p => KeyValuePair.Create(p.Key, (JsonNode?)WithoutNulls(p.Value!)))),
            JsonArray a => new JsonArray(a.Select(i => i == null ? null : WithoutNulls(i)).ToArray()),
            _ => node.DeepClone(),
        };

        public static bool Contains(JsonNode? actual, JsonNode? wanted) => (actual, wanted) switch
        {
            (_, null) => true,
            (JsonObject a, JsonObject w) => w.All(p => p.Value == null || (a[p.Key] is JsonNode got && Contains(got, p.Value))),
            (JsonArray a, JsonArray w) => a.Count == w.Count && a.Zip(w).All(x => Contains(x.First, x.Second)),
            (null, _) => false,
            (JsonNode a, JsonNode w) => JsonNode.DeepEquals(a, w),
        };
    }
}
