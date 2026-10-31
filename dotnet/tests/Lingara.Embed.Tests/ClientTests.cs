using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading;
using Lingara.Embed.Events;
using Xunit;

namespace Lingara.Embed.Tests
{
    /// <summary>The client over the real host library, offline (ADR 1.10.26aa D12).</summary>
    public class ClientTests
    {
        // 1.10.26aa AC17
        [Fact]
        public void DrainGrowsTheBufferAndKeepsOrder()
        {
            // A 16-byte poll buffer: every event is larger than it, so each one grows it.
            using var client = LingaraEmbedClient.Create(Support.Offline(), Native.NativeSelection.For(NativeLibraryMode.Dynamic), 16);
            var requests = Enumerable.Range(0, 3).Select(_ => client.DialogueSay(client.DialogueOpen(Support.Mei()), "hi")).ToList();
            Thread.Sleep(300);
            client.TokenUnavailable("offline");
            Thread.Sleep(300);

            var first = new List<EmbedEvent>();
            Assert.Equal(2, client.Drain(first, max: 2));
            Assert.IsType<EmbedEvent.TokenNeeded>(first[0]);
            Assert.IsType<EmbedEvent.Failed>(first[1]);

            var rest = new List<EmbedEvent>();
            Assert.Equal(2, client.Drain(rest));
            var answered = first.Skip(1).Concat(rest).Cast<EmbedEvent.Failed>().Select(f => f.Error.Request!.Value).ToList();
            Assert.Equal(requests.OrderBy(r => r.Value), answered.OrderBy(r => r.Value));
            Assert.False(client.TryPoll(out _));
        }

        // 1.10.26aa AC18
        [Fact]
        public void ARefusalThrowsTheCoresErrorAndALargeAnswerArrivesWhole()
        {
            using var client = LingaraEmbedClient.Create(Support.Offline());
            var refused = Assert.Throws<EmbedException>(() => client.PracticeAnswer(new SessionId(999), "x"));
            Assert.Equal((ErrorKind.UnknownId, false, null), (refused.Kind, refused.Retryable, refused.NativeStatus));
            Assert.False(string.IsNullOrEmpty(refused.Error.Message));

            // Twelve 480-character lines: a snapshot answer well past the 4 KiB buffer.
            var window = Enumerable.Range(0, 12)
                .Select(i => new WindowEntry { Speaker = i % 2 == 0 ? Speaker.Player : Speaker.Npc, Text = new string((char)('a' + i), 480) })
                .ToArray();
            var dialogue = client.DialogueOpen(Support.Mei(window));
            var snapshot = client.GetDialogueSnapshot(dialogue)!;
            Assert.Equal(12, snapshot.Window.Count);
            Assert.Equal(window.Select(w => (w.Speaker, w.Text)), snapshot.Window.Select(w => (w.Speaker, w.Text)));
            Assert.Equal(("Mei", DialoguePhase.Idle), (snapshot.Npc.Name, snapshot.Phase));
        }

        // 30.10.26p D4: EmbedVersion reaches the core's config (a malformed one is its
        // Config refusal), and a ProvideToken carrying the notice says it once.
        [Fact]
        public void TheEmbedVersionReachesTheCoreAndTheNoticeIsSaidOnce()
        {
            var options = Support.Offline();
            options.EmbedVersion = "v1.0";
            var refused = Assert.Throws<EmbedException>(() => LingaraEmbedClient.Create(options));
            Assert.Equal(ErrorKind.Config, refused.Kind);

            options.EmbedVersion = "1.1.0";
            using var client = LingaraEmbedClient.Create(options);
            var notice = new UpdateAvailable { Latest = "1.2.0", Deprecated = true };
            client.ProvideToken("lgr_et_fetchedTokenValue0001", "2099-01-01T00:00:00Z", notice);
            client.ProvideToken("lgr_et_fetchedTokenValue0002", "2099-01-01T00:00:00Z", notice);
            Thread.Sleep(200);
            var events = new List<EmbedEvent>();
            client.Drain(events);
            var told = Assert.Single(events.OfType<EmbedEvent.UpdateAvailable>());
            Assert.Equal(("1.2.0", true, (string?)null), (told.Latest, told.Deprecated, told.Message));
        }

        // 1.10.26aa AC19
        [Fact]
        public void DisposeFreesOnceAndLaterCallsThrow()
        {
            var fake = new FakeNative();
            var client = LingaraEmbedClient.Create(Support.Offline(), fake, LingaraEmbedClient.PollBufferBytes);
            client.Dispose();
            client.Dispose();
            Assert.Equal(1, fake.FreeCalls);
            Assert.Throws<ObjectDisposedException>(() => client.StreamPause());
            Assert.Throws<ObjectDisposedException>(() => client.TryPoll(out _));
            Assert.Throws<ObjectDisposedException>(() => client.Status);

            // A leaked client is freed by its finalizer, once; a disposed one never again.
            Leak(fake);
            GC.Collect();
            GC.WaitForPendingFinalizers();
            GC.Collect();
            Assert.Equal(2, fake.FreeCalls);
        }

        private static void Leak(FakeNative fake) => LingaraEmbedClient.Create(Support.Offline(), fake, LingaraEmbedClient.PollBufferBytes);
    }
}
