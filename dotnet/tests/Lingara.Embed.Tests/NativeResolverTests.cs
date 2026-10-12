using System;
using System.IO;
using System.Runtime.InteropServices;
using Lingara.Embed.Native;
using Xunit;

namespace Lingara.Embed.Tests
{
    public class NativeResolverTests
    {
        private static string HostLibrary()
        {
            if (OperatingSystem.IsWindows())
            {
                return "lingara_embed.dll";
            }

            return OperatingSystem.IsMacOS() ? "liblingara_embed.dylib" : "liblingara_embed.so";
        }

        private static string PackageRid()
        {
            if (OperatingSystem.IsWindows())
            {
                return "win-x64";
            }

            return OperatingSystem.IsMacOS() ? "osx" : "linux-x64";
        }

        // 1.10.26aa AC20
        [Fact]
        public void TheResolverFindsTheRidNativeBesideTheAssembly()
        {
            var game = Path.Combine(Path.GetTempPath(), "lingara-resolver-" + Guid.NewGuid().ToString("N"));
            try
            {
                // A directory with no library at all: the probe misses, and says so with zero.
                Directory.CreateDirectory(game);
                Assert.Equal(IntPtr.Zero, NativeResolver.LoadBeside(game));

                // The package's layout, runtimes/<rid>/native/, beside the "assembly".
                var native = Path.Combine(game, "runtimes", PackageRid(), "native");
                Directory.CreateDirectory(native);
                File.Copy(Path.Combine(AppContext.BaseDirectory, HostLibrary()), Path.Combine(native, HostLibrary()));
                var handle = NativeResolver.LoadBeside(game);
                Assert.NotEqual(IntPtr.Zero, handle);
                Assert.NotEqual(IntPtr.Zero, NativeLibrary.GetExport(handle, "lingara_embed_abi_version"));
            }
            finally
            {
                try
                {
                    Directory.Delete(game, recursive: true);
                }
                catch (IOException)
                {
                    // A loaded library cannot be deleted on Windows; the temp directory keeps it.
                }
                catch (UnauthorizedAccessException)
                {
                }
            }
        }
    }
}
