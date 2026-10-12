using System;
using System.Linq;
using System.Net;
using System.Net.Http;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using Lingara.Embed.Server.Http;
using Xunit;

namespace Lingara.Embed.Server.Tests;

public class TokenCacheTests
{
    /// <summary>A token endpoint that numbers its grants and can hold an answer back.</summary>
    private sealed class Grants : HttpMessageHandler
    {
        private int count;

        internal int Count => count;

        internal int ExpiresIn { get; set; } = 3600;

        internal TaskCompletionSource? Hold { get; set; }

        protected override async Task<HttpResponseMessage> SendAsync(HttpRequestMessage request, CancellationToken ct)
        {
            var n = Interlocked.Increment(ref count);
            Assert.Equal("Basic", request.Headers.Authorization?.Scheme);
            if (Hold is { } hold)
            {
                await hold.Task.WaitAsync(ct);
            }

            var json = $"{{\"access_token\":\"lgr_at_{n}\",\"token_type\":\"Bearer\",\"expires_in\":{ExpiresIn}}}";
            return new HttpResponseMessage(HttpStatusCode.OK) { Content = new StringContent(json, Encoding.UTF8, "application/json") };
        }
    }

    private static TokenCache Cache(FixedTime time, HttpClient http) =>
        new(new Transport(http, KitVersion.UserAgent, time), new Uri("http://127.0.0.1/oauth/token"), "lgr_cid_test", "lgr_cs_…", null, TimeSpan.FromSeconds(30));

    /// <summary>
    /// 1.10.26af AC30: the token cache refreshes <c>min(60 s, expires_in / 2)</c>
    /// before expiry, shares one exchange between concurrent callers, and
    /// clears only the token a 401 was answered with.
    /// </summary>
    [Fact]
    public async Task RefreshesEarlySharesOneFlightAndClearsOnlyItsOwn()
    {
        var start = DateTimeOffset.FromUnixTimeSeconds(1_790_000_000);
        var time = new FixedTime(start);
        var grants = new Grants();
        using var http = new HttpClient(grants);
        using var cache = Cache(time, http);

        // A 3600 s token is fresh until 60 s before it expires.
        Assert.Equal("lgr_at_1", await cache.GetAsync(default));
        time.Now = start.AddSeconds(3539);
        Assert.Equal("lgr_at_1", await cache.GetAsync(default));
        time.Now = start.AddSeconds(3540);
        Assert.Equal("lgr_at_2", await cache.GetAsync(default));

        // A 30 s token is stale at half its life, not on arrival.
        grants.ExpiresIn = 30;
        time.Now = start.AddSeconds(3600 + 3540);
        Assert.Equal("lgr_at_3", await cache.GetAsync(default));
        time.Now = time.Now.AddSeconds(14);
        Assert.Equal("lgr_at_3", await cache.GetAsync(default));
        time.Now = time.Now.AddSeconds(1);
        Assert.Equal("lgr_at_4", await cache.GetAsync(default));

        // Ten concurrent callers share one exchange.
        grants.ExpiresIn = 3600;
        cache.Invalidate("lgr_at_4");
        grants.Hold = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var waiting = Enumerable.Range(0, 10).Select(_ => cache.GetAsync(default)).ToArray();
        grants.Hold.SetResult();
        var tokens = await Task.WhenAll(waiting);
        Assert.Equal(5, grants.Count);
        Assert.All(tokens, t => Assert.Equal("lgr_at_5", t));
        grants.Hold = null;

        // A 401 answered with an older token leaves the newer one cached.
        cache.Invalidate("lgr_at_4");
        Assert.Equal("lgr_at_5", await cache.GetAsync(default));
        Assert.Equal(5, grants.Count);
        cache.Invalidate("lgr_at_5");
        Assert.Equal("lgr_at_6", await cache.GetAsync(default));
    }
}
