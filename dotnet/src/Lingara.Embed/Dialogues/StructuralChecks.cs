using System;
using System.Collections.Generic;
using System.Linq;

namespace Lingara.Embed.Dialogues
{
    /// <summary>
    /// D5's checks: what needs no grammar, under N2 §10's rule numbers. The predicate grammar,
    /// the length bounds and the byte cap stay the core's and are not copied here.
    /// </summary>
    internal static class StructuralChecks
    {
        private const string Abandoned = "abandoned";

        public static void Run(DialogueBuilder d)
        {
            var ends = Ends(d);
            Ids(d, ends);
            Unique(d.Npcs.Select((n, i) => ($"npcs[{i}].id", n.Id)));
            Unique(d.Facts.Select((f, i) => ($"facts[{i}].key", f.Key)));
            Unique(d.Endings.Select((e, i) => ($"endings[{i}].id", e.Id)));
            Unique(d.Checkpoints.Select((c, i) => ($"checkpoints[{i}].id", c.Id)));
            Unique(Edges(d).Select(e => ($"{e.Path}.id", e.Edge.Id)));
            Targets(d, ends);
            Disclosed(d);
            Defaults(d);
            Levels(d);
            WhenLessCycle(d, ends);
        }

        /// <summary>
        /// What an edge may end on (ADR 3.11.26g §11.1): the declared endings, or v1's two when none is
        /// declared, plus the built-in <c>abandoned</c>. The crate's rule, held to it by AC20.
        /// </summary>
        private static HashSet<string> Ends(DialogueBuilder d)
        {
            var ends = new HashSet<string>(d.DeclaredEndings().Select(e => e.Id), StringComparer.Ordinal) { Abandoned };
            return ends;
        }

        private static ArgumentException Fault(string path, string message) => new ArgumentException($"{path}: {message}");

        private static IEnumerable<(string Path, DialogueEdge Edge)> Edges(DialogueBuilder d) =>
            d.Checkpoints.SelectMany((c, i) => c.Edges.Select((e, j) => ($"checkpoints[{i}].edges[{j}]", e)));

        /// <summary><c>^[a-z][a-z0-9_]{0,47}$</c>, written out.</summary>
        private static bool IsId(string value) =>
            value.Length >= 1 && value.Length <= 48 && value[0] >= 'a' && value[0] <= 'z'
            && value.All(c => (c >= 'a' && c <= 'z') || (c >= '0' && c <= '9') || c == '_');

        private static void Id(string value, string path)
        {
            if (!IsId(value))
            {
                throw Fault(path, $"\"{value}\" is not an id (^[a-z][a-z0-9_]{{0,47}}$)");
            }
        }

        private static void Ids(DialogueBuilder d, HashSet<string> ends)
        {
            Id(d.Id, "id");
            Id(d.Start, "start");
            for (var i = 0; i < d.Endings.Count; i++)
            {
                Id(d.Endings[i].Id, $"endings[{i}].id");
                if (d.Endings[i].Id == Abandoned)
                {
                    throw Fault($"endings[{i}].id", "\"abandoned\" is built in, so it is never declared");
                }
            }

            for (var i = 0; i < d.Npcs.Count; i++)
            {
                Id(d.Npcs[i].Id, $"npcs[{i}].id");
            }

            for (var i = 0; i < d.Facts.Count; i++)
            {
                Id(d.Facts[i].Key, $"facts[{i}].key");
            }

            for (var i = 0; i < d.Checkpoints.Count; i++)
            {
                Id(d.Checkpoints[i].Id, $"checkpoints[{i}].id");
                if (ends.Contains(d.Checkpoints[i].Id))
                {
                    throw Fault($"checkpoints[{i}].id", $"\"{d.Checkpoints[i].Id}\" is an ending's id");
                }
            }

            foreach (var (path, edge) in Edges(d))
            {
                Id(edge.Id, $"{path}.id");
            }
        }

        /// <summary>Rule 1: unique within each list, and edge ids across the whole dialogue.</summary>
        private static void Unique(IEnumerable<(string Path, string Id)> ids)
        {
            var seen = new HashSet<string>(StringComparer.Ordinal);
            foreach (var (path, id) in ids)
            {
                if (!seen.Add(id))
                {
                    throw Fault(path, $"repeats the id \"{id}\"");
                }
            }
        }

        /// <summary>Rule 2: <c>start</c> names a checkpoint, and every <c>to</c> a checkpoint or an ending.</summary>
        private static void Targets(DialogueBuilder d, HashSet<string> ends)
        {
            var ids = new HashSet<string>(d.Checkpoints.Select(c => c.Id), StringComparer.Ordinal);
            if (!ids.Contains(d.Start))
            {
                throw Fault("start", $"names no checkpoint: \"{d.Start}\"");
            }

            foreach (var (path, edge) in Edges(d))
            {
                if (!ends.Contains(edge.To) && !ids.Contains(edge.To))
                {
                    throw Fault($"{path}.to", $"names no checkpoint or ending: \"{edge.To}\"");
                }
            }
        }

        /// <summary>Rule 3, for <c>may_disclose</c>.</summary>
        private static void Disclosed(DialogueBuilder d)
        {
            var keys = new HashSet<string>(d.Facts.Select(f => f.Key), StringComparer.Ordinal);
            for (var i = 0; i < d.Checkpoints.Count; i++)
            {
                var k = d.Checkpoints[i].MayDisclose.ToList().FindIndex(key => !keys.Contains(key));
                if (k >= 0)
                {
                    throw Fault($"checkpoints[{i}].may_disclose[{k}]", $"unknown fact \"{d.Checkpoints[i].MayDisclose[k]}\"");
                }
            }
        }

        /// <summary>Rule 5, and D2 rules 3–4's builder range for a number.</summary>
        private static void Defaults(DialogueBuilder d)
        {
            for (var i = 0; i < d.Facts.Count; i++)
            {
                var fact = d.Facts[i];
                var ok = fact.Default switch
                {
                    null => true,
                    bool _ => fact.Type == FactType.Boolean,
                    string _ => fact.Type == FactType.String,
                    double x => fact.Type == FactType.Number && CanonicalJson.Number(x) != null,
                    _ => false,
                };
                if (!ok)
                {
                    throw Fault($"facts[{i}].default", "must match the fact's type, and a number must be within the builders' range");
                }
            }
        }

        private static void Levels(DialogueBuilder d)
        {
            for (var i = 0; i < d.Npcs.Count; i++)
            {
                var languages = d.Npcs[i].Languages ?? Array.Empty<NpcLanguage>();
                for (var j = 0; j < languages.Count; j++)
                {
                    if (languages[j].Speaking < 1 || languages[j].Speaking > 9 || languages[j].Comprehension < 1 || languages[j].Comprehension > 9)
                    {
                        throw Fault($"npcs[{i}].languages[{j}]", "levels run from 1 to 9");
                    }
                }
            }
        }

        /// <summary>Rule 6: no cycle made only of edges with no <c>when</c>, which would loop with no turn.</summary>
        private static void WhenLessCycle(DialogueBuilder d, HashSet<string> ends)
        {
            var next = d.Checkpoints.ToDictionary(
                c => c.Id,
                c => c.Edges.Where(e => e.When == null && !ends.Contains(e.To)).Select(e => e.To).ToList(),
                StringComparer.Ordinal);
            var state = new Dictionary<string, int>(StringComparer.Ordinal); // 1 on the path, 2 done
            foreach (var id in next.Keys)
            {
                if (Visit(id, next, state))
                {
                    throw Fault($"checkpoints[{d.Checkpoints.FindIndex(c => c.Id == id)}]", "closes a cycle of edges without `when`, which would loop with no turn");
                }
            }
        }

        private static bool Visit(string id, Dictionary<string, List<string>> next, Dictionary<string, int> state)
        {
            if (state.TryGetValue(id, out var s))
            {
                return s == 1;
            }

            state[id] = 1;
            var cycle = next[id].Any(to => Visit(to, next, state));
            state[id] = 2;
            return cycle;
        }
    }
}
