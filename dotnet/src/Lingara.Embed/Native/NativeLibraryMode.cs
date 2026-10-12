using System.Runtime.InteropServices;

namespace Lingara.Embed
{
    /// <summary>
    /// Which import class the client binds at its first <c>Create</c> (ADR
    /// 1.10.26aa D8). <see cref="Auto"/> picks <see cref="StaticInternal"/> on iOS
    /// and <see cref="Dynamic"/> elsewhere; Unity's snippet sets
    /// <see cref="StaticInternal"/> itself under <c>UNITY_IOS</c>, since IL2CPP's
    /// answer to the runtime check is not relied on.
    /// </summary>
    public enum NativeLibraryMode
    {
        Auto,
        Dynamic,
        StaticInternal,
    }
}

namespace Lingara.Embed.Native
{
    internal static class NativeSelection
    {
        public static INativeApi For(NativeLibraryMode mode) => Resolve(mode) == NativeLibraryMode.StaticInternal
            ? NativeMethodsInternal.Api
            : NativeMethodsDynamic.Api;

        public static NativeLibraryMode Resolve(NativeLibraryMode mode)
        {
            if (mode != NativeLibraryMode.Auto)
            {
                return mode;
            }

            return RuntimeInformation.IsOSPlatform(OSPlatform.Create("IOS")) ? NativeLibraryMode.StaticInternal : NativeLibraryMode.Dynamic;
        }
    }
}
