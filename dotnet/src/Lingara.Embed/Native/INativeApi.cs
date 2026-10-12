using System;

namespace Lingara.Embed.Native
{
    /// <summary>
    /// The nine functions of <c>lingara_embed.h</c> (ADR 1.10.26aa D3), behind one
    /// interface so the import class is chosen once (D8) and a test can stand in a
    /// fake. Every signature is blittable: pointers, <see cref="UIntPtr"/> for
    /// <c>size_t</c>, and 32-bit integers.
    /// </summary>
    internal unsafe interface INativeApi
    {
        uint AbiVersion();

        IntPtr Version();

        int New(uint abiMajor, byte* configJson, UIntPtr configLen, IntPtr* outHandle);

        void Free(IntPtr handle);

        int Execute(IntPtr handle, byte* commandJson, UIntPtr commandLen, byte* buf, UIntPtr cap, UIntPtr* outLen);

        int Poll(IntPtr handle, byte* buf, UIntPtr cap, UIntPtr* outLen);

        int Status(IntPtr handle, byte* buf, UIntPtr cap, UIntPtr* outLen);

        int LastAnswer(byte* buf, UIntPtr cap, UIntPtr* outLen);

        int LastError(byte* buf, UIntPtr cap, UIntPtr* outLen);
    }

    /// <summary>The header's status codes and ABI version (D3, D5).</summary>
    internal static class NativeStatus
    {
        public const uint AbiMajor = 1;
        public const uint AbiMinor = 0;

        public const int Ok = 0;
        public const int Empty = 1;
        public const int BufferTooSmall = 2;
        public const int Refused = 3;
        public const int ErrNull = -1;
        public const int ErrInvalidUtf8 = -2;
        public const int ErrInvalidJson = -3;
        public const int ErrUnknownCommand = -4;
        public const int ErrInvalidArgument = -5;
        public const int ErrClosed = -6;
        public const int ErrPanic = -7;
        public const int ErrAbiMismatch = -8;
    }
}
