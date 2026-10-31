using System;
using System.Collections.Generic;
using System.Diagnostics.CodeAnalysis;
using System.Runtime.InteropServices;
using Lingara.Embed.Commands;
using Lingara.Embed.Events;
using Lingara.Embed.Json;
using Lingara.Embed.Native;

namespace Lingara.Embed
{
    /// <summary>
    /// One player's connection to Lingara (ADR 1.10.26aa D8), over the C ABI.
    /// Create one per player, call <see cref="Drain"/> once a frame from the main
    /// thread, and answer <c>TokenNeeded</c> with <see cref="ProvideToken"/>.
    /// Commands may be called from any thread; each answers in the call.
    /// </summary>
    public sealed unsafe class LingaraEmbedClient : IDisposable
    {
        internal const int AnswerBufferBytes = 4 * 1024;
        internal const int PollBufferBytes = 16 * 1024;

        [ThreadStatic]
        private static byte[]? answerBuffer;

        private readonly INativeApi api;
        private readonly EmbedHandle handle;
        private readonly bool keepRaw;
        private readonly object pollLock = new object();
        private byte[] pollBuffer;
        private bool disposed;

        private LingaraEmbedClient(INativeApi api, EmbedHandle handle, bool keepRaw, int pollBytes)
        {
            this.api = api;
            this.handle = handle;
            this.keepRaw = keepRaw;
            pollBuffer = new byte[pollBytes];
        }

        private unsafe delegate int Fill(byte* buf, UIntPtr cap, UIntPtr* outLen);

        /// <summary>Which import class to bind. Set it before the first <see cref="Create(EmbedOptions)"/>.</summary>
        public static NativeLibraryMode NativeMode { get; set; }

        /// <summary>K1's <c>status()</c>: the stream, its cursor, whether a token is needed.</summary>
        public EmbedStatus Status
        {
            get
            {
                using var lease = Lease();
                var raw = lease.Raw;
                return TypeDecoders.Status(Parse(ReadGrowing(PollBufferBytes, (b, c, l) => UnsafeStatus(raw, b, c, l))));
            }
        }

        /// <summary>Starts the core. Throws <see cref="EmbedException"/> on an ABI mismatch or a refused config.</summary>
        public static LingaraEmbedClient Create(EmbedOptions options)
        {
            var mode = NativeSelection.Resolve(NativeMode);
#if NET8_0_OR_GREATER
            if (mode == NativeLibraryMode.Dynamic)
            {
                NativeResolver.Install();
            }
#endif
            return Create(options, NativeSelection.For(mode), PollBufferBytes);
        }

        internal static unsafe LingaraEmbedClient Create(EmbedOptions options, INativeApi api, int pollBytes)
        {
            CheckAbi(api);
            var config = options.ToConfigJson("lingara-dotnet/" + (Marshal.PtrToStringAnsi(api.Version()) ?? "0.0.0"));
            IntPtr raw;
            int status;
            fixed (byte* c = config)
            {
                status = api.New(NativeStatus.AbiMajor, c, (UIntPtr)config.Length, &raw);
            }

            if (status == NativeStatus.Refused)
            {
                AnswerDecoder.Outcome(status, ReadGrowing(AnswerBufferBytes, (b, cap, l) => api.LastAnswer(b, cap, l)));
            }

            if (status != NativeStatus.Ok)
            {
                throw Fault(api, status);
            }

            return new LingaraEmbedClient(api, new EmbedHandle(api, raw), options.KeepRawJson, pollBytes);
        }

        // ── Tokens (K1 D4) ────────────────────────────────────────────

        /// <summary>
        /// The handoff body's fields, verbatim: an <c>lgr_et_</c> token, its RFC 3339
        /// expiry and, when the handoff carries one, its <c>update_available</c>
        /// (ADR 30.10.26p), which the core turns into one <c>UpdateAvailable</c> event.
        /// </summary>
        public void ProvideToken(string token, string expiresAt, UpdateAvailable? updateAvailable = null) =>
            AnswerDecoder.Done(Execute(CommandEncoder.ProvideToken(token, expiresAt, updateAvailable)));

        /// <summary>A token with its lifetime from the mint's <c>expires_in</c>.</summary>
        public void ProvideTokenFor(string token, uint expiresInSecs, UpdateAvailable? updateAvailable = null) =>
            AnswerDecoder.Done(Execute(CommandEncoder.ProvideTokenFor(token, expiresInSecs, updateAvailable)));

        /// <summary>The game cannot get a token: every waiting call fails now, not at the timeout.</summary>
        public void TokenUnavailable(string reason) => AnswerDecoder.Done(Execute(CommandEncoder.TokenUnavailable(reason)));

        /// <summary>Switches to, or re-points, the Http token source (D6).</summary>
        public void SetTokenEndpoint(string url, IReadOnlyList<Header> headers) =>
            AnswerDecoder.Done(Execute(CommandEncoder.SetTokenEndpoint(url, headers)));

        // ── Stream (K1 D5) and world (K1 D6) ─────────────────────────

        public void StreamPause() => AnswerDecoder.Done(Execute(CommandEncoder.StreamPause()));

        public void StreamResume() => AnswerDecoder.Done(Execute(CommandEncoder.StreamResume()));

        /// <summary>Sends <c>world.context_changed</c>; <c>WorldSent</c> answers with this id.</summary>
        public RequestId SendContext(WorldContext context) => new RequestId(AnswerDecoder.Id(Execute(CommandEncoder.SendContext(context))));

        // ── Practice (K1 D7) ──────────────────────────────────────────

        public SessionId PracticeStart(PracticeSource source) => new SessionId(AnswerDecoder.Id(Execute(CommandEncoder.PracticeStart(source))));

        public PracticeSnapshot? GetPracticeSnapshot(SessionId session) => AnswerDecoder.Practice(Execute(CommandEncoder.PracticeSnapshot(session)));

        public PracticeSnapshot PracticeAnswer(SessionId session, string answer) => Snapshot(CommandEncoder.PracticeAnswer(session, answer));

        public PracticeSnapshot PracticeChoose(SessionId session, uint option) => Snapshot(CommandEncoder.PracticeChoose(session, option));

        public PracticeSnapshot PracticeReveal(SessionId session) => Snapshot(CommandEncoder.PracticeReveal(session));

        public PracticeSnapshot PracticeSelfGrade(SessionId session, bool correct) => Snapshot(CommandEncoder.PracticeSelfGrade(session, correct));

        public PracticeSnapshot PracticeNext(SessionId session) => Snapshot(CommandEncoder.PracticeNext(session));

        public PracticeSnapshot PracticeFinish(SessionId session) => Snapshot(CommandEncoder.PracticeFinish(session));

        // ── Dialogue (K1 D8) ──────────────────────────────────────────

        public DialogueId DialogueOpen(DialogueConfig config) => new DialogueId(AnswerDecoder.Id(Execute(CommandEncoder.DialogueOpen(config))));

        /// <summary>One line to the NPC; its reply streams as <c>NpcTurn</c> events carrying this id.</summary>
        public RequestId DialogueSay(DialogueId dialogue, string line) => new RequestId(AnswerDecoder.Id(Execute(CommandEncoder.DialogueSay(dialogue, line))));

        public void DialogueSetLevel(DialogueId dialogue, byte level) => AnswerDecoder.Done(Execute(CommandEncoder.DialogueSetLevel(dialogue, level)));

        public void DialogueSetScene(DialogueId dialogue, string? scene) => AnswerDecoder.Done(Execute(CommandEncoder.DialogueSetScene(dialogue, scene)));

        public void DialogueCancel(DialogueId dialogue) => AnswerDecoder.Done(Execute(CommandEncoder.DialogueCancel(dialogue)));

        public void DialogueClose(DialogueId dialogue) => AnswerDecoder.Done(Execute(CommandEncoder.DialogueClose(dialogue)));

        public DialogueSnapshot? GetDialogueSnapshot(DialogueId dialogue) => AnswerDecoder.Dialogue(Execute(CommandEncoder.DialogueSnapshot(dialogue)));

        /// <summary>
        /// Loads a <c>dialogue.v1</c> graph, as text, and opens a run of it (ADR 28.10.26f);
        /// <c>CheckpointReached</c> follows.
        /// </summary>
        public DialogueId DialogueLoad(string irJson, GraphSession session) => new DialogueId(AnswerDecoder.Id(Execute(CommandEncoder.DialogueLoad(irJson, session))));

        /// <summary>A dialogue built with <see cref="Dialogues.DialogueBuilder"/>, sent as its canonical text (ADR 28.10.26g D6).</summary>
        public DialogueId DialogueLoad(Dialogues.Dialogue ir, GraphSession session) => DialogueLoad(ir.ToCanonicalJson(), session);

        /// <summary>From either source (ADR 28.10.26i): a stored one answers its id in <c>Loading</c>, then <c>DialogueLoaded</c> or <c>DialogueLoadFailed</c>.</summary>
        public DialogueId DialogueLoad(DialogueSource source, GraphSession session) => new DialogueId(AnswerDecoder.Id(Execute(CommandEncoder.DialogueLoad(source, session))));

        /// <summary>Sets a fact the graph declared.</summary>
        public void DialogueSetFact(DialogueId dialogue, string key, FactValue value) => AnswerDecoder.Done(Execute(CommandEncoder.DialogueSetFact(dialogue, key, value)));

        /// <summary>Checks a <c>dialogue.v1</c> document as <see cref="DialogueLoad(string, GraphSession)"/> would, creating nothing.</summary>
        public void DialogueValidate(string irJson) => AnswerDecoder.Done(Execute(CommandEncoder.DialogueValidate(irJson)));

        // ── The queue ─────────────────────────────────────────────────

        /// <summary>One event, or false when the queue is empty.</summary>
        public bool TryPoll([NotNullWhen(true)] out EmbedEvent? evt)
        {
            lock (pollLock)
            {
                using var lease = Lease();
                var raw = lease.Raw;
                while (true)
                {
                    var status = PollInto(raw, out var len);
                    if (status == NativeStatus.Empty)
                    {
                        evt = null;
                        return false;
                    }

                    if (status == NativeStatus.BufferTooSmall)
                    {
                        // The FFI kept the event in its stash: a bigger buffer gets the same one.
                        pollBuffer = new byte[NextPowerOfTwo(len)];
                        continue;
                    }

                    evt = status == NativeStatus.Ok ? Decode(len) : throw Fault(api, status);
                    return true;
                }
            }
        }

        /// <summary>The per-frame call: up to <paramref name="max"/> events into <paramref name="into"/>, in queue order.</summary>
        public int Drain(List<EmbedEvent> into, int max = 256)
        {
            var count = 0;
            while (count < max && TryPoll(out var evt))
            {
                into.Add(evt);
                count++;
            }

            return count;
        }

        public void Dispose()
        {
            disposed = true;
            handle.Dispose();
        }

        private static void CheckAbi(INativeApi api)
        {
            var version = api.AbiVersion();
            var (major, minor) = (version >> 16, version & 0xffff);
            if (major != NativeStatus.AbiMajor || minor < NativeStatus.AbiMinor)
            {
                var message = $"the native library is ABI {major}.{minor}; this Lingara.Embed needs ABI {NativeStatus.AbiMajor}.{NativeStatus.AbiMinor} or a later {NativeStatus.AbiMajor}.x";
                throw new EmbedException(new EmbedError { Kind = ErrorKind.Config, KindName = "config", Message = message }, NativeStatus.ErrAbiMismatch);
            }
        }

        /// <summary>A negative status, with the FFI's own text from <c>last_error</c>.</summary>
        private static unsafe EmbedException Fault(INativeApi api, int status)
        {
            var bytes = ReadGrowing(AnswerBufferBytes, (b, c, l) => api.LastError(b, c, l));
            var text = bytes.Length == 0 ? $"status {status}" : System.Text.Encoding.UTF8.GetString(bytes);
            var (kind, name) = status switch
            {
                NativeStatus.ErrAbiMismatch => (ErrorKind.Config, "config"),
                NativeStatus.ErrClosed => (ErrorKind.Closed, "closed"),
                _ => (ErrorKind.Internal, "internal"),
            };
            return new EmbedException(new EmbedError { Kind = kind, KindName = name, Message = text }, status);
        }

        /// <summary>Calls <paramref name="fill"/> until its answer fits; empty when it has none.</summary>
        private static unsafe byte[] ReadGrowing(int initial, Fill fill)
        {
            var buf = new byte[initial];
            while (true)
            {
                UIntPtr len;
                int status;
                fixed (byte* b = buf)
                {
                    status = fill(b, (UIntPtr)buf.Length, &len);
                }

                if (status == NativeStatus.BufferTooSmall)
                {
                    buf = new byte[NextPowerOfTwo(len)];
                    continue;
                }

                return status == NativeStatus.Empty || status < 0 ? Array.Empty<byte>() : buf.AsSpan(0, (int)len).ToArray();
            }
        }

        private static int NextPowerOfTwo(UIntPtr needed)
        {
            var size = 1;
            while ((ulong)size < (ulong)needed)
            {
                size <<= 1;
            }

            return size;
        }

        private static JsonValue Parse(byte[] json) =>
            JsonReader.TryParse(json, out var value, out var error) ? value : throw new EmbedException("the core answered unreadable JSON: " + error);

        private unsafe int UnsafeStatus(IntPtr raw, byte* b, UIntPtr c, UIntPtr* l) => api.Status(raw, b, c, l);

        private unsafe int PollInto(IntPtr raw, out UIntPtr len)
        {
            UIntPtr got;
            int status;
            fixed (byte* b = pollBuffer)
            {
                status = api.Poll(raw, b, (UIntPtr)pollBuffer.Length, &got);
            }

            len = got;
            return status;
        }

        private EmbedEvent Decode(UIntPtr len)
        {
            try
            {
                return EventDecoder.Decode(pollBuffer.AsSpan(0, (int)len), keepRaw);
            }
            catch (JsonFormatException e)
            {
                throw new EmbedException("the core sent an event this package cannot read: " + e.Message, e);
            }
        }

        private EmbedHandle.Lease Lease()
        {
#if NET8_0_OR_GREATER
            ObjectDisposedException.ThrowIf(disposed, this);
#else
            if (disposed)
            {
                throw new ObjectDisposedException(nameof(LingaraEmbedClient));
            }
#endif
            return handle.Acquire();
        }

        private PracticeSnapshot Snapshot(byte[] command) =>
            AnswerDecoder.Practice(Execute(command)) ?? throw new EmbedException("the core answered a practice command with no snapshot");

        /// <summary>Runs one command and returns its outcome; the core's refusal is thrown.</summary>
        private unsafe JsonValue Execute(byte[] command)
        {
            using var lease = Lease();
            var buf = answerBuffer ??= new byte[AnswerBufferBytes];
            UIntPtr len;
            int status;
            fixed (byte* c = command)
            fixed (byte* b = buf)
            {
                status = api.Execute(lease.Raw, c, (UIntPtr)command.Length, b, (UIntPtr)buf.Length, &len);
            }

            if (status == NativeStatus.BufferTooSmall)
            {
                // The command has run: fetch its answer, never run it twice (D3).
                buf = answerBuffer = new byte[NextPowerOfTwo(len)];
                fixed (byte* b = buf)
                {
                    status = api.LastAnswer(b, (UIntPtr)buf.Length, &len);
                }
            }

            if (status < 0)
            {
                throw Fault(api, status);
            }

            try
            {
                return AnswerDecoder.Outcome(status, buf.AsSpan(0, (int)len));
            }
            catch (JsonFormatException e)
            {
                throw new EmbedException("the core answered with an unreadable outcome: " + e.Message, e);
            }
        }
    }
}
