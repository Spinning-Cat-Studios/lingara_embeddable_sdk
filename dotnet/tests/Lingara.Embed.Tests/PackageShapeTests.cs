using System;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Xml.Linq;
using Xunit;

namespace Lingara.Embed.Tests
{
    /// <summary>
    /// The packed nupkg (ADR 1.10.26aa D8, D9). `make test-dotnet` packs it over
    /// stand-in natives first and passes its path as LINGARA_EMBED_NUPKG; run any
    /// other way this test fails, never skips, so it cannot pass on no package.
    /// </summary>
    public class PackageShapeTests
    {
        private static readonly string[] Natives =
        {
            "runtimes/win-x64/native/lingara_embed.dll",
            "runtimes/osx/native/liblingara_embed.dylib",
            "runtimes/linux-x64/native/liblingara_embed.so",
            "runtimes/android-arm64/native/liblingara_embed.so",
            "runtimes/ios-arm64/native/liblingara_embed.a",
        };

        // 1.10.26aa AC22
        [Fact]
        public void ThePackageHasNoDependenciesAndFiveNatives()
        {
            var path = Environment.GetEnvironmentVariable("LINGARA_EMBED_NUPKG");
            Assert.False(string.IsNullOrEmpty(path), "LINGARA_EMBED_NUPKG is unset: run this through `make test-dotnet`");
            using var nupkg = ZipFile.OpenRead(path!);
            var entries = nupkg.Entries.Select(e => e.FullName).ToList();

            Assert.Equal(Natives.OrderBy(n => n), entries.Where(e => e.StartsWith("runtimes/", StringComparison.Ordinal)).OrderBy(n => n));
            Assert.Contains("lib/netstandard2.1/Lingara.Embed.dll", entries);
            Assert.Contains("lib/net8.0/Lingara.Embed.dll", entries);
            Assert.Contains("README.md", entries);

            var nuspec = nupkg.Entries.Single(e => e.FullName.EndsWith(".nuspec", StringComparison.Ordinal));
            using var stream = nuspec.Open();
            var dependencies = XDocument.Load(stream).Descendants().Where(e => e.Name.LocalName == "dependency").ToList();
            Assert.Empty(dependencies);

            // The player package holds no server half: no Server namespace, no mint, no HMAC.
            var types = typeof(LingaraEmbedClient).Assembly.GetTypes();
            Assert.DoesNotContain(types, t => (t.Namespace ?? string.Empty).StartsWith("Lingara.Embed.Server", StringComparison.Ordinal));
            var members = types.SelectMany(t => t.GetMembers()).Select(m => m.Name).ToList();
            Assert.DoesNotContain(members, m => m.Contains("Mint", StringComparison.OrdinalIgnoreCase) || m.Contains("Hmac", StringComparison.OrdinalIgnoreCase));
            Assert.DoesNotContain(types, t => t.Name.Contains("Hmac", StringComparison.OrdinalIgnoreCase) || t.Name.Contains("Webhook", StringComparison.Ordinal));
        }
    }
}
