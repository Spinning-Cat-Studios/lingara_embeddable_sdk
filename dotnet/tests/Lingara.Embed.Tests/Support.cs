using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Text;
using System.Threading;
using Lingara.Embed.Events;
using Lingara.Embed.Native;

namespace Lingara.Embed.Tests
{
    /// <summary>What every test file shares: the linked vectors, an offline client, a poller, a fake ABI.</summary>
    internal static class Support
    {
        public static string Vector(string name) => Path.Combine(AppContext.BaseDirectory, "vectors", name);

        /// <summary>Nothing listens here, and nothing needs a token at start (D12).</summary>
        public static EmbedOptions Offline(bool keepRaw = false) => new EmbedOptions
        {
            BaseUrl = "http://127.0.0.1:9",
            StreamOnStart = false,
            KeepRawJson = keepRaw,
        };

        public static DialogueConfig Mei(params WindowEntry[] window) => new DialogueConfig
        {
            Npc = new Npc { Name = "Mei" },
            SourceLang = "en",
            TargetLang = "zh",
            Level = 2,
            Window = window,
        };

        /// <summary>Polls until <paramref name="match"/>, returning it and everything before it.</summary>
        public static (EmbedEvent Found, List<EmbedEvent> Before) PollUntil(LingaraEmbedClient client, Func<EmbedEvent, bool> match, int withinMs = 5000)
        {
            var before = new List<EmbedEvent>();
            var clock = Stopwatch.StartNew();
            while (clock.ElapsedMilliseconds < withinMs)
            {
                if (!client.TryPoll(out var evt))
                {
                    Thread.Sleep(5);
                    continue;
                }

                if (match(evt))
                {
                    return (evt, before);
                }

                before.Add(evt);
            }

            throw new TimeoutException($"no matching event within {withinMs} ms; saw {before.Count}");
        }
    }

    /// <summary>
    /// A stand-in for the C ABI: a fixed ABI version, a queue of event JSON, and a
    /// count of every <c>new</c> and <c>free</c>. Commands answer <c>done</c>.
    /// </summary>
    internal sealed unsafe class FakeNative : INativeApi
    {
        private static readonly byte[] VersionBytes = Encoding.ASCII.GetBytes("0.0.0\0");
        private readonly Queue<byte[]> events = new Queue<byte[]>();
        private readonly GCHandleBox version = new GCHandleBox(VersionBytes);

        public uint Abi { get; set; } = NativeStatus.AbiMajor << 16;

        public int NewCalls { get; private set; }

        public int FreeCalls { get; private set; }

        public void Enqueue(string json) => events.Enqueue(Encoding.UTF8.GetBytes(json));

        public uint AbiVersion() => Abi;

        public IntPtr Version() => version.Pointer;

        public int New(uint abiMajor, byte* configJson, UIntPtr configLen, IntPtr* outHandle)
        {
            NewCalls++;
            *outHandle = new IntPtr(0x1000);
            return NativeStatus.Ok;
        }

        public void Free(IntPtr handle) => FreeCalls++;

        public int Execute(IntPtr handle, byte* commandJson, UIntPtr commandLen, byte* buf, UIntPtr cap, UIntPtr* outLen) =>
            Write(Encoding.UTF8.GetBytes("{\"type\":\"done\"}"), buf, cap, outLen, NativeStatus.Ok);

        public int Poll(IntPtr handle, byte* buf, UIntPtr cap, UIntPtr* outLen)
        {
            if (events.Count == 0)
            {
                *outLen = UIntPtr.Zero;
                return NativeStatus.Empty;
            }

            var status = Write(events.Peek(), buf, cap, outLen, NativeStatus.Ok);
            if (status == NativeStatus.Ok)
            {
                events.Dequeue();
            }

            return status;
        }

        public int Status(IntPtr handle, byte* buf, UIntPtr cap, UIntPtr* outLen) =>
            Write(Encoding.UTF8.GetBytes("{\"stream\":{\"type\":\"paused\"},\"cursor\":null,\"needs_token\":true,\"closed\":false}"), buf, cap, outLen, NativeStatus.Ok);

        public int LastAnswer(byte* buf, UIntPtr cap, UIntPtr* outLen)
        {
            *outLen = UIntPtr.Zero;
            return NativeStatus.Empty;
        }

        public int LastError(byte* buf, UIntPtr cap, UIntPtr* outLen)
        {
            *outLen = UIntPtr.Zero;
            return NativeStatus.Empty;
        }

        private static int Write(byte[] bytes, byte* buf, UIntPtr cap, UIntPtr* outLen, int fits)
        {
            *outLen = (UIntPtr)bytes.Length;
            if ((ulong)bytes.Length > (ulong)cap)
            {
                return NativeStatus.BufferTooSmall;
            }

            for (var i = 0; i < bytes.Length; i++)
            {
                buf[i] = bytes[i];
            }

            return fits;
        }

        /// <summary>A pinned copy of a static string, as the ABI's <c>version()</c> is.</summary>
        private sealed class GCHandleBox
        {
            private readonly System.Runtime.InteropServices.GCHandle handle;

            public GCHandleBox(byte[] bytes) => handle = System.Runtime.InteropServices.GCHandle.Alloc(bytes, System.Runtime.InteropServices.GCHandleType.Pinned);

            public IntPtr Pointer => handle.AddrOfPinnedObject();
        }
    }
}
