using System;

namespace Lingara.Embed.Dialogues
{
    /// <summary>
    /// Levels as a builder takes them (ADR 28.10.26g D3): the IR stores 1–9, and a label is
    /// accepted only where it maps offline. HSK, TOPIK and General labels <em>are</em> their
    /// number, so they have no helper. A helper does not check the language: <c>Cefr("A2")</c>
    /// on <c>zh</c> stores 2. Under <c>subtle</c> the NPC speaks at the lower of its speaking
    /// level and the learner's; under <c>none</c>, at its own.
    /// </summary>
    public static class Level
    {
        /// <summary>The top of the scale: what an NPC with no profile is treated as.</summary>
        public const byte Native = 9;

        private static readonly string[] CefrLabels = { "A1", "A2", "B1", "B2", "C1", "C2" };

        // From the easiest: N5 is 1, which is the mistake this helper prevents.
        private static readonly string[] JlptLabels = { "N5", "N4", "N3", "N2", "N1" };

        /// <summary><c>A1</c>–<c>C2</c> as 1–6.</summary>
        /// <exception cref="ArgumentException">Not a CEFR label.</exception>
        public static byte Cefr(string label) => Position(CefrLabels, label, "CEFR");

        /// <summary><c>N5</c>–<c>N1</c> as 1–5.</summary>
        /// <exception cref="ArgumentException">Not a JLPT label.</exception>
        public static byte Jlpt(string label) => Position(JlptLabels, label, "JLPT");

        private static byte Position(string[] labels, string label, string scale)
        {
            var i = Array.IndexOf(labels, label);
            if (i < 0)
            {
                throw new ArgumentException($"\"{label}\" is not a {scale} level ({string.Join(", ", labels)})", nameof(label));
            }

            return (byte)(i + 1);
        }
    }
}
