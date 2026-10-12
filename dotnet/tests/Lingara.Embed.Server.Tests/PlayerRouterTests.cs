using System;
using System.Collections.Generic;
using System.Threading;
using System.Threading.Tasks;
using Xunit;

namespace Lingara.Embed.Server.Tests;

public class PlayerRouterTests
{
    /// <summary>A directory that counts lookups, and fails them when asked to.</summary>
    private sealed class Counting : IPlayerDirectory
    {
        internal readonly MemoryPlayerDirectory Inner = new();

        internal int Lookups { get; private set; }

        internal bool Fail { get; init; }

        public ValueTask RememberAsync(string playerRef, string subject, CancellationToken ct = default) => Inner.RememberAsync(playerRef, subject, ct);

        public ValueTask<string?> PlayerRefForAsync(string subject, CancellationToken ct = default)
        {
            Lookups++;
            return Fail ? throw new InvalidOperationException("the store is down") : Inner.PlayerRefForAsync(subject, ct);
        }

        public ValueTask ForgetAsync(string playerRef, CancellationToken ct = default) => Inner.ForgetAsync(playerRef, ct);
    }

    /// <summary>The router over a vector's verifier at its own <c>now</c>, with a directory knowing <c>lgr_sub_conformance</c> as <c>p1</c>.</summary>
    private static async Task<(PlayerRouter Router, Counting Directory, List<Exception> Errors)> Router(SignatureVector v, bool fail = false)
    {
        var directory = new Counting { Fail = fail };
        await directory.Inner.RememberAsync("p1", "lgr_sub_conformance");
        var errors = new List<Exception>();
        return (new PlayerRouter(directory, v.Verifier()) { OnError = errors.Add }, directory, errors);
    }

    /// <summary>
    /// 1.10.26af AC21: through <c>HandleAsync</c> with a header lookup, a
    /// tampered body is <c>401</c> and never reaches the directory or
    /// <c>onEvent</c>; a valid event reaches <c>onEvent</c> with the
    /// remembered <c>player_ref</c>; an unknown subject arrives with none; a
    /// raising handler and a raising directory are each <c>500</c> and reach
    /// <c>OnError</c>; a valid signature over a malformed payload is <c>400</c>.
    /// </summary>
    [Fact]
    public async Task HandleVerifiesBeforeResolving()
    {
        var seen = new List<string?>();
        Task Record(RoutedEvent routed, CancellationToken ct)
        {
            seen.Add(routed.PlayerRef);
            return Task.CompletedTask;
        }

        var tampered = SignatureVector.Named("tampered-body");
        var (r, dir, _) = await Router(tampered);
        Assert.Equal(401, await r.HandleAsync(tampered.Body, tampered.Header, Record));
        Assert.Equal((0, 0), (dir.Lookups, seen.Count));

        var valid = SignatureVector.Named("valid-single-signature");
        (r, dir, _) = await Router(valid);
        Assert.Equal(204, await r.HandleAsync(valid.Body, valid.Header, Record));
        Assert.Equal(1, dir.Lookups);
        Assert.Equal(new string?[] { "p1" }, seen);

        await dir.Inner.ForgetAsync("p1");
        Assert.Equal(204, await r.HandleAsync(valid.Body, valid.Header, Record));
        Assert.Equal(new string?[] { "p1", null }, seen);

        var (raising, _, errors) = await Router(valid);
        Assert.Equal(500, await raising.HandleAsync(valid.Body, valid.Header, (_, _) => throw new InvalidOperationException("the handler broke")));
        Assert.Equal("the handler broke", Assert.Single(errors).Message);

        seen.Clear();
        (r, _, errors) = await Router(valid, fail: true);
        Assert.Equal(500, await r.HandleAsync(valid.Body, valid.Header, Record));
        Assert.Equal("the store is down", Assert.Single(errors).Message);
        Assert.Empty(seen);

        var notJson = SignatureVector.Named("body-not-json");
        (r, _, _) = await Router(notJson);
        Assert.Equal(400, await r.HandleAsync(notJson.Body, notJson.Header, Record));
        Assert.Empty(seen);
    }
}
