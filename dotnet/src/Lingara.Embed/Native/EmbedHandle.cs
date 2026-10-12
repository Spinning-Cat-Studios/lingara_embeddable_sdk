using System;
using System.Runtime.InteropServices;

namespace Lingara.Embed.Native
{
    /// <summary>
    /// Owns the <c>LingaraEmbed*</c> (ADR 1.10.26aa D8): a leaked client is freed
    /// by the finalizer, <c>Dispose</c> frees exactly once, and a call that holds
    /// <see cref="Lease"/> keeps the pointer alive across a concurrent dispose.
    /// </summary>
    internal sealed class EmbedHandle : SafeHandle
    {
        private readonly INativeApi api;

        public EmbedHandle(INativeApi api, IntPtr raw)
            : base(IntPtr.Zero, ownsHandle: true)
        {
            this.api = api;
            SetHandle(raw);
        }

        public override bool IsInvalid => handle == IntPtr.Zero;

        /// <summary>Adds a reference for one call; throws <see cref="ObjectDisposedException"/> once disposed.</summary>
        public Lease Acquire()
        {
            var added = false;
            DangerousAddRef(ref added);
            return new Lease(this, added);
        }

        protected override bool ReleaseHandle()
        {
            api.Free(handle);
            return true;
        }

        /// <summary>The raw pointer for the length of one call.</summary>
        public readonly struct Lease : IDisposable
        {
            private readonly EmbedHandle owner;
            private readonly bool added;

            public Lease(EmbedHandle owner, bool added)
            {
                this.owner = owner;
                this.added = added;
            }

            public IntPtr Raw => owner.DangerousGetHandle();

            public void Dispose()
            {
                if (added)
                {
                    owner.DangerousRelease();
                }
            }
        }
    }
}
