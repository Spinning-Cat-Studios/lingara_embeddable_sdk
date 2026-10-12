using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Threading;
using Lingara.Embed;
using Lingara.Embed.Events;

// C's offline lifecycle (ffi/tests/c/smoke.c) through the public C# API, under
// NativeAOT (ADR 1.10.26aa D12, AC23). Exit 0 only when every step held.
using var client = LingaraEmbedClient.Create(new EmbedOptions { BaseUrl = "http://127.0.0.1:9", StreamOnStart = false });

var dialogue = client.DialogueOpen(new DialogueConfig { Npc = new Npc { Name = "Mei" }, SourceLang = "en", TargetLang = "zh", Level = 2 });
var session = client.PracticeStart(new PracticeSource.Plan { PlanId = "8f0e2c1a-0000-4000-8000-000000000001", Report = false });

var events = new List<EmbedEvent>();
var clock = Stopwatch.StartNew();
EmbedEvent.TokenNeeded? need = null;
while (need == null && clock.ElapsedMilliseconds < 5000)
{
    events.Clear();
    client.Drain(events);
    need = events.Find(e => e is EmbedEvent.TokenNeeded) as EmbedEvent.TokenNeeded;
    Thread.Sleep(5);
}

if (need == null || !client.Status.NeedsToken)
{
    Console.Error.WriteLine("aot-smoke: no token_needed, or status does not need a token");
    return 1;
}

Console.WriteLine($"aot-smoke: dialogue {dialogue}, session {session}, token_needed {need.Request} ({need.Reason}): ok");
return 0;
