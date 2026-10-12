using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.Text;
using Lingara.Embed.Events;
using Lingara.Embed.Native;

namespace Lingara.Embed.Bench
{
    /// <summary>
    /// Times <c>Drain</c> of 256 queued <c>npc_turn</c> fragment events (ADR
    /// 1.10.26aa D12): the C# half of the per-event JSON cost, the poll loop and
    /// the decoder, over a native stand-in that serves the queue from memory.
    /// Above 1 ms on a CI runner, D2 is revisited.
    /// </summary>
    internal static class DrainBench
    {
        private const int Events = 256;
        private const int Rounds = 2000;

        public static int Main()
        {
            var native = new QueueNative();
            using var client = LingaraEmbedClient.Create(new EmbedOptions(), native, LingaraEmbedClient.PollBufferBytes);
            var into = new List<EmbedEvent>(Events);
            for (var warm = 0; warm < 200; warm++)
            {
                Round(client, native, into);
            }

            var samples = new double[Rounds];
            for (var i = 0; i < Rounds; i++)
            {
                var clock = Stopwatch.StartNew();
                Round(client, native, into);
                samples[i] = clock.Elapsed.TotalMilliseconds;
            }

            Array.Sort(samples);
            var median = samples[Rounds / 2];
            var p99 = samples[Rounds * 99 / 100];
            Console.WriteLine(string.Format(CultureInfo.InvariantCulture, "drain of {0} npc_turn fragments: median {1:F3} ms, p99 {2:F3} ms", Events, median, p99));
            return 0;
        }

        private static void Round(LingaraEmbedClient client, QueueNative native, List<EmbedEvent> into)
        {
            into.Clear();
            native.Fill(Events);
            if (client.Drain(into, Events) != Events)
            {
                throw new InvalidOperationException("the drain fell short");
            }
        }

        /// <summary>A native stand-in whose queue holds <c>n</c> copies of one fragment event.</summary>
        private sealed unsafe class QueueNative : INativeApi
        {
            private static readonly byte[] Fragment = Encoding.UTF8.GetBytes(
                "{\"type\":\"npc_turn\",\"dialogue\":4,\"request\":5,\"update\":{\"type\":\"fragment\",\"text\":\"Ni hao, welcome to the tea house. What would you like today?\"}}");

            private static readonly byte[] Version = Encoding.ASCII.GetBytes("0.0.0\0");
            private readonly System.Runtime.InteropServices.GCHandle version = System.Runtime.InteropServices.GCHandle.Alloc(Version, System.Runtime.InteropServices.GCHandleType.Pinned);
            private int queued;

            public void Fill(int n) => queued = n;

            public uint AbiVersion() => NativeStatus.AbiMajor << 16;

            IntPtr INativeApi.Version() => version.AddrOfPinnedObject();

            public int New(uint abiMajor, byte* configJson, UIntPtr configLen, IntPtr* outHandle)
            {
                *outHandle = new IntPtr(0x1000);
                return NativeStatus.Ok;
            }

            public void Free(IntPtr handle)
            {
            }

            public int Execute(IntPtr handle, byte* commandJson, UIntPtr commandLen, byte* buf, UIntPtr cap, UIntPtr* outLen) => NativeStatus.ErrNull;

            public int Poll(IntPtr handle, byte* buf, UIntPtr cap, UIntPtr* outLen)
            {
                if (queued == 0)
                {
                    *outLen = UIntPtr.Zero;
                    return NativeStatus.Empty;
                }

                queued--;
                *outLen = (UIntPtr)Fragment.Length;
                Fragment.AsSpan().CopyTo(new Span<byte>(buf, (int)cap));
                return NativeStatus.Ok;
            }

            public int Status(IntPtr handle, byte* buf, UIntPtr cap, UIntPtr* outLen) => NativeStatus.ErrNull;

            public int LastAnswer(byte* buf, UIntPtr cap, UIntPtr* outLen) => NativeStatus.Empty;

            public int LastError(byte* buf, UIntPtr cap, UIntPtr* outLen) => NativeStatus.Empty;
        }
    }
}
