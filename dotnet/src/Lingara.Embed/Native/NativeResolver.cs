#if NET8_0_OR_GREATER
using System;
using System.Diagnostics.CodeAnalysis;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;

namespace Lingara.Embed.Native
{
    /// <summary>
    /// The fallback probe (ADR 1.10.26aa D8, D11): when the default probe misses,
    /// look in <c>runtimes/&lt;rid&gt;/native/</c> beside the assembly. It covers
    /// hosts that load the game assembly into their own load context without its
    /// <c>deps.json</c>, Godot's editor being the known case.
    /// </summary>
    internal static class NativeResolver
    {
        private static int installed;

        public static void Install()
        {
            if (System.Threading.Interlocked.Exchange(ref installed, 1) == 0)
            {
                NativeLibrary.SetDllImportResolver(typeof(NativeResolver).Assembly, Resolve);
            }
        }

        private static IntPtr Resolve(string libraryName, Assembly assembly, DllImportSearchPath? searchPath)
        {
            if (libraryName != "lingara_embed")
            {
                return IntPtr.Zero;
            }

            if (NativeLibrary.TryLoad(libraryName, assembly, searchPath, out var loaded))
            {
                return loaded;
            }

            return LoadBeside(AssemblyDirectory(assembly));
        }

        // Empty in a single-file or NativeAOT app, where the base directory is the answer anyway.
        [UnconditionalSuppressMessage("SingleFile", "IL3000", Justification = "An empty Location falls back to AppContext.BaseDirectory.")]
        private static string AssemblyDirectory(Assembly assembly)
        {
            var location = assembly.Location;
            return string.IsNullOrEmpty(location) ? AppContext.BaseDirectory : Path.GetDirectoryName(location) ?? AppContext.BaseDirectory;
        }

        /// <summary>The library under <paramref name="directory"/>'s <c>runtimes/</c>, or zero.</summary>
        public static IntPtr LoadBeside(string directory)
        {
            foreach (var rid in Rids())
            {
                var path = Path.Combine(directory, "runtimes", rid, "native", FileName());
                if (File.Exists(path) && NativeLibrary.TryLoad(path, out var handle))
                {
                    return handle;
                }
            }

            return IntPtr.Zero;
        }

        private static string FileName()
        {
            if (OperatingSystem.IsWindows())
            {
                return "lingara_embed.dll";
            }

            return OperatingSystem.IsMacOS() ? "liblingara_embed.dylib" : "liblingara_embed.so";
        }

        /// <summary>The package's RID first, then this process's own, for a local build.</summary>
        private static string[] Rids()
        {
            if (OperatingSystem.IsWindows())
            {
                return new[] { "win-x64" };
            }

            if (OperatingSystem.IsMacOS())
            {
                return new[] { "osx", RuntimeInformation.RuntimeIdentifier };
            }

            return OperatingSystem.IsAndroid() ? new[] { "android-arm64" } : new[] { "linux-x64", RuntimeInformation.RuntimeIdentifier };
        }
    }
}
#endif
