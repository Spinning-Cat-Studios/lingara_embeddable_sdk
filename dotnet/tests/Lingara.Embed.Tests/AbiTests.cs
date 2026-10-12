using Lingara.Embed.Native;
using Xunit;

namespace Lingara.Embed.Tests
{
    public class AbiTests
    {
        // 1.10.26aa AC13
        [Fact]
        public void AnotherAbiMajorIsRefusedWithBothVersions()
        {
            var fake = new FakeNative { Abi = (NativeStatus.AbiMajor + 1) << 16 };
            var refused = Assert.Throws<EmbedException>(() => LingaraEmbedClient.Create(Support.Offline(), fake, LingaraEmbedClient.PollBufferBytes));
            Assert.Equal(ErrorKind.Config, refused.Kind);
            Assert.Equal(NativeStatus.ErrAbiMismatch, refused.NativeStatus);
            Assert.Contains($"ABI {NativeStatus.AbiMajor + 1}.0", refused.Error.Message, System.StringComparison.Ordinal);
            Assert.Contains($"ABI {NativeStatus.AbiMajor}.{NativeStatus.AbiMinor}", refused.Error.Message, System.StringComparison.Ordinal);
            Assert.Equal(0, fake.NewCalls);

            // The real library this package was built beside is accepted.
            using var client = LingaraEmbedClient.Create(Support.Offline());
            Assert.True(client.Status.NeedsToken);
        }
    }
}
