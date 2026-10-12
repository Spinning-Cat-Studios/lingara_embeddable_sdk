using System;

namespace Lingara.Embed
{
    /// <summary>
    /// A command the core refused, or a call that never reached it (ADR 1.10.26aa
    /// D8). <see cref="Error"/> is K1's <c>EmbedError</c>. A call that never
    /// reached the core also sets <see cref="NativeStatus"/>, with
    /// <c>Kind</c> <c>Config</c> (an ABI mismatch), <c>Closed</c> (a poisoned
    /// handle) or <c>Internal</c>. Asynchronous failures never throw: they are
    /// <c>EmbedEvent.Failed</c> in the queue.
    /// </summary>
    public sealed class EmbedException : Exception
    {
        public EmbedException(EmbedError error)
            : base(error.ToString())
        {
            Error = error;
        }

        public EmbedException(EmbedError error, int nativeStatus)
            : base(error.ToString())
        {
            Error = error;
            NativeStatus = nativeStatus;
        }

        public EmbedException()
            : this(new EmbedError { Kind = ErrorKind.Internal, KindName = "internal", Message = "unknown failure" })
        {
        }

        public EmbedException(string message)
            : this(new EmbedError { Kind = ErrorKind.Internal, KindName = "internal", Message = message })
        {
        }

        public EmbedException(string message, Exception innerException)
            : base(message, innerException)
        {
            Error = new EmbedError { Kind = ErrorKind.Internal, KindName = "internal", Message = message };
        }

        public EmbedError Error { get; }

        /// <summary>The ABI's negative status, when the call never reached the core.</summary>
        public int? NativeStatus { get; }

        public ErrorKind Kind => Error.Kind;

        public bool Retryable => Error.Retryable;
    }
}
