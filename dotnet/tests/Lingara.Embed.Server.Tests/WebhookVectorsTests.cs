using System;
using Lingara.Embed.Server.Webhooks;
using Xunit;

namespace Lingara.Embed.Server.Tests;

public class WebhookVectorsTests
{
    /// <summary>
    /// 1.10.26af AC29: every E7 vector in <c>webhook-signatures.json</c> gives
    /// its expected answer through <c>Verify</c> at the vector's own <c>now</c>
    /// (<c>ok</c> with its id, type and <c>unknown</c>; an error's reason; a
    /// refused construction), and <c>VerifySignature</c> follows E7's AC45
    /// rule: it passes every <c>ok</c> and <c>malformed_payload</c> vector and
    /// raises every other error vector's own reason.
    /// </summary>
    [Fact]
    public void EverySharedVectorVerifiesAsExpected()
    {
        var all = SignatureVector.All();
        Assert.True(all.Count >= 25, "the shared vectors are all here");
        foreach (var v in all)
        {
            if (v.Expect.TryGetProperty("refused", out _))
            {
                Assert.Throws<ArgumentException>(() => v.Verifier());
                continue;
            }

            var verifier = v.Verifier();
            if (v.Expect.TryGetProperty("ok", out var ok))
            {
                var got = verifier.Verify(v.Body, v.Header);
                Assert.True(ok.GetProperty("id").GetString() == got.Id, v.Name);
                Assert.True(ok.GetProperty("type").GetString() == got.Type, v.Name);
                var unknown = ok.TryGetProperty("unknown", out var u) && u.GetBoolean();
                Assert.True(unknown == got.IsUnknown, $"{v.Name}: unknown");
                verifier.VerifySignature(v.Body, v.Header);
                continue;
            }

            var reason = v.Expect.GetProperty("error").GetString();
            var error = Assert.Throws<WebhookVerificationException>(() => verifier.Verify(v.Body, v.Header));
            Assert.True(reason == error.Reason, $"{v.Name}: {error.Reason}");
            if (reason == WebhookVerificationException.MalformedPayload)
            {
                verifier.VerifySignature(v.Body, v.Header);
            }
            else
            {
                var signature = Assert.Throws<WebhookVerificationException>(() => verifier.VerifySignature(v.Body, v.Header));
                Assert.True(reason == signature.Reason, $"{v.Name}: VerifySignature {signature.Reason}");
            }
        }
    }
}
