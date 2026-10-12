using System;
using System.Runtime.InteropServices;

namespace Lingara.Embed.Native
{
    /// <summary>
    /// <c>lingara_embed.h</c> imported from the running process, <c>__Internal</c>
    /// (ADR 1.10.26aa D8): iOS, where the static library is linked into the app.
    /// Hand-written, and held to the header by <c>NativeSurfaceTests</c>. IL2CPP
    /// compiles it on every platform; off iOS it is simply never called.
    /// </summary>
    internal static unsafe class NativeMethodsInternal
    {
        private const string Lib = "__Internal";

        public static readonly INativeApi Api = new Bound();

        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern uint lingara_embed_abi_version();

        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern IntPtr lingara_embed_version();

        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern int lingara_embed_new(uint abi_major, byte* config_json, UIntPtr config_len, IntPtr* out_handle);

        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern void lingara_embed_free(IntPtr handle);

        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern int lingara_embed_execute(IntPtr handle, byte* command_json, UIntPtr command_len, byte* buf, UIntPtr cap, UIntPtr* out_len);

        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern int lingara_embed_poll(IntPtr handle, byte* buf, UIntPtr cap, UIntPtr* out_len);

        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern int lingara_embed_status(IntPtr handle, byte* buf, UIntPtr cap, UIntPtr* out_len);

        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern int lingara_embed_last_answer(byte* buf, UIntPtr cap, UIntPtr* out_len);

        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern int lingara_embed_last_error(byte* buf, UIntPtr cap, UIntPtr* out_len);

        private sealed class Bound : INativeApi
        {
            public uint AbiVersion() => lingara_embed_abi_version();

            public IntPtr Version() => lingara_embed_version();

            public int New(uint abiMajor, byte* configJson, UIntPtr configLen, IntPtr* outHandle) =>
                lingara_embed_new(abiMajor, configJson, configLen, outHandle);

            public void Free(IntPtr handle) => lingara_embed_free(handle);

            public int Execute(IntPtr handle, byte* commandJson, UIntPtr commandLen, byte* buf, UIntPtr cap, UIntPtr* outLen) =>
                lingara_embed_execute(handle, commandJson, commandLen, buf, cap, outLen);

            public int Poll(IntPtr handle, byte* buf, UIntPtr cap, UIntPtr* outLen) => lingara_embed_poll(handle, buf, cap, outLen);

            public int Status(IntPtr handle, byte* buf, UIntPtr cap, UIntPtr* outLen) => lingara_embed_status(handle, buf, cap, outLen);

            public int LastAnswer(byte* buf, UIntPtr cap, UIntPtr* outLen) => lingara_embed_last_answer(buf, cap, outLen);

            public int LastError(byte* buf, UIntPtr cap, UIntPtr* outLen) => lingara_embed_last_error(buf, cap, outLen);
        }
    }
}
