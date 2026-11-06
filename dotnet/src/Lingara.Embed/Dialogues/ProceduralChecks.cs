using System;
using System.Collections.Generic;
using System.Linq;

namespace Lingara.Embed.Dialogues
{
    /// <summary>
    /// A procedural document's structural checks (ADR 3.11.26j D8), at the crate's paths and rule words: what
    /// needs no grammar. The predicates, the bounds and the rest of spec rules 1–7 stay the core's.
    /// </summary>
    internal static class ProceduralChecks
    {
        public static void Run(ProceduralBuilder p)
        {
            var cast = p.Cast;
            Dispositions(cast.Npcs);
            if (cast.Endings.Count == 0)
            {
                throw Fault("endings", "missing_member", "a procedural dialogue declares at least one ending");
            }

            Unique(cast.Npcs.Select((n, i) => ($"npcs[{i}].id", n.Id)));
            Unique(p.Topics.Select((t, i) => ($"procedural.topics[{i}].id", t.Id)));
            var facts = new HashSet<string>(cast.Facts.Select(f => f.Key), StringComparer.Ordinal);
            var pool = new HashSet<string>(p.DisclosableKeys, StringComparer.Ordinal);
            for (var i = 0; i < p.DisclosableKeys.Count; i++)
            {
                if (!facts.Contains(p.DisclosableKeys[i]))
                {
                    throw Fault($"procedural.disclosable[{i}]", "undeclared_fact", $"\"{p.DisclosableKeys[i]}\" is not a declared fact");
                }
            }

            Discloses(p.Topics, pool);
            Atoms(p);
        }

        /// <summary>Each NPC's disposition, 0–1 on both axes: the crate's <c>number_range</c>.</summary>
        public static void Dispositions(IEnumerable<DialogueNpc> npcs)
        {
            foreach (var (npc, i) in npcs.Select((n, i) => (n, i)).Where(x => x.n.Disposition != null))
            {
                var d = npc.Disposition!;
                foreach (var (name, x) in new[] { ("suggestibility", d.Suggestibility), ("initiative", d.Initiative) })
                {
                    if (double.IsNaN(x) || x < 0 || x > 1)
                    {
                        throw Fault($"npcs[{i}].disposition.{name}", "number_range", "must be between 0 and 1");
                    }
                }
            }
        }

        private static ArgumentException Fault(string path, string rule, string message) => new ArgumentException($"{path}: {message} ({rule})");

        private static void Unique(IEnumerable<(string Path, string Id)> ids)
        {
            var seen = new HashSet<string>(StringComparer.Ordinal);
            foreach (var (path, id) in ids.Where(x => !seen.Add(x.Id)))
            {
                throw Fault(path, "duplicate_id", $"repeats the id \"{id}\"");
            }
        }

        /// <summary>A topic discloses only keys of the pool (the crate's <c>pool_escape</c>).</summary>
        private static void Discloses(List<ProceduralTopic> topics, HashSet<string> pool)
        {
            for (var i = 0; i < topics.Count; i++)
            {
                for (var j = 0; j < topics[i].Discloses.Count; j++)
                {
                    if (!pool.Contains(topics[i].Discloses[j]))
                    {
                        throw Fault($"procedural.topics[{i}].discloses[{j}]", "pool_escape", $"\"{topics[i].Discloses[j]}\" is outside `disclosable`");
                    }
                }
            }
        }

        /// <summary>Unique atoms, <c>open</c> and <c>close</c> among them, and a Reveal only with a topic that discloses something.</summary>
        private static void Atoms(ProceduralBuilder p)
        {
            var atoms = p.AtomList.Select(a => a.Atom).ToList();
            if (atoms.Distinct().Count() != atoms.Count || !atoms.Contains(AtomKind.Open) || !atoms.Contains(AtomKind.Close))
            {
                throw Fault("procedural.atoms", "atom_set", "lists each atom once, open and close among them");
            }

            var reveal = atoms.IndexOf(AtomKind.Reveal);
            if (reveal >= 0 && !p.Topics.Any(t => t.Discloses.Count > 0))
            {
                throw Fault($"procedural.atoms[{reveal}]", "atom_set", "\"reveal\" has nothing to work on");
            }
        }
    }
}
