using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json;

namespace Lingara.Embed.Server.Webhooks;

/// <summary>
/// E7 D4 step 6's last check without generated models: a known type whose
/// <c>data</c> does not decode is <c>malformed_payload</c>. The table is the
/// catalogue's data schemas reduced to required keys, primitive types and
/// enums, the same reduction the TypeScript library generates
/// (<c>generated/events.ts</c>' <c>SHAPES</c>, at <c>LIBRARY_FLOOR</c>). A
/// type absent here is unknown, and routed rather than refused.
/// </summary>
internal static class EventShapes
{
    private const string Table = """
    {
      "lesson_plan.ready": {"type":["object"],"properties":{"plan_id":{"type":["string"]},"status":{"type":["string"],"enum":["complete","partial"]},"title":{"type":["string","null"]},"source_lang":{"type":["string"]},"target_lang":{"type":["string"]},"level":{"type":["integer"]}},"required":["plan_id","status","title","source_lang","target_lang","level"]},
      "lesson_plan.failed": {"type":["object"],"properties":{"plan_id":{"type":["string"]},"reason":{"type":["string"],"enum":["generation_failed","timed_out"]}},"required":["plan_id","reason"]},
      "usage.threshold_reached": {"type":["object"],"properties":{"scope":{"type":["string"],"enum":["account","client"]},"threshold_pct":{"type":["integer"],"enum":[50,70,90,100]},"month":{"type":["string"]},"client_id":{"type":["string","null"]}},"required":["scope","threshold_pct","month"]},
      "webhook.test": {"type":["object"]},
      "app.installed": {"type":["object"],"properties":{"client_id":{"type":["string"]},"install_id":{"type":["string"]},"context":{"type":["array"],"items":{"type":["string"],"enum":["languages","plan_summary","review_due","tutor_topic"]}},"tutor_note":{"type":["boolean"]}},"required":["client_id","install_id","context","tutor_note"]},
      "app.uninstalled": {"type":["object"],"properties":{"client_id":{"type":["string"]},"install_id":{"type":["string"]}},"required":["client_id","install_id"]},
      "app.disabled": {"type":["object"],"properties":{"client_id":{"type":["string"]},"install_id":{"type":["string"]}},"required":["client_id","install_id"]},
      "app.enabled": {"type":["object"],"properties":{"client_id":{"type":["string"]},"install_id":{"type":["string"]}},"required":["client_id","install_id"]},
      "practice.completed": {"type":["object"],"properties":{"plan_id":{"type":["string"]},"items":{"type":["array"],"items":{"type":["object"],"properties":{"set_number":{"type":["integer"]},"question":{"type":["integer"]},"outcome":{"type":["string"],"enum":["correct","incorrect","revealed","self_correct","self_incorrect","skipped"]}},"required":["set_number","question","outcome"]}},"completed":{"type":["boolean"]}},"required":["plan_id","items","completed"]}
    }
    """;

    private static readonly JsonElement Shapes = JsonDocument.Parse(Table).RootElement.Clone();

    /// <summary>Whether <paramref name="type"/> is in the catalogue this kit knows.</summary>
    internal static bool IsKnown(string type) => Shapes.TryGetProperty(type, out _);

    /// <summary>Whether <paramref name="data"/> decodes as <paramref name="type"/>'s data. Unknown types always do.</summary>
    internal static bool Conforms(string type, JsonElement data) =>
        !Shapes.TryGetProperty(type, out var shape) || Matches(shape, data);

    private static bool Matches(JsonElement shape, JsonElement value)
    {
        if (shape.TryGetProperty("enum", out var choices) && !choices.EnumerateArray().Any(c => Same(c, value)))
        {
            return false;
        }

        if (shape.TryGetProperty("type", out var types) && !types.EnumerateArray().Any(t => IsType(t.GetString()!, value)))
        {
            return false;
        }

        return value.ValueKind switch
        {
            JsonValueKind.Array => !shape.TryGetProperty("items", out var items) || value.EnumerateArray().All(v => Matches(items, v)),
            JsonValueKind.Object => Members(shape, value),
            _ => true,
        };
    }

    /// <summary>Every required key present, and each present property matching (an optional null is absent).</summary>
    private static bool Members(JsonElement shape, JsonElement value)
    {
        var required = shape.TryGetProperty("required", out var r) ? r.EnumerateArray().Select(k => k.GetString()!).ToHashSet(StringComparer.Ordinal) : new HashSet<string>(StringComparer.Ordinal);
        if (!required.All(k => value.TryGetProperty(k, out _)))
        {
            return false;
        }

        if (!shape.TryGetProperty("properties", out var properties))
        {
            return true;
        }

        return properties.EnumerateObject().All(p =>
            !value.TryGetProperty(p.Name, out var v)
            || (v.ValueKind == JsonValueKind.Null && !required.Contains(p.Name))
            || Matches(p.Value, v));
    }

    private static bool IsType(string type, JsonElement value) => type switch
    {
        "null" => value.ValueKind == JsonValueKind.Null,
        "array" => value.ValueKind == JsonValueKind.Array,
        "object" => value.ValueKind == JsonValueKind.Object,
        "boolean" => value.ValueKind is JsonValueKind.True or JsonValueKind.False,
        "string" => value.ValueKind == JsonValueKind.String,
        "number" => value.ValueKind == JsonValueKind.Number,
        "integer" => value.ValueKind == JsonValueKind.Number && value.TryGetDecimal(out var d) && decimal.Truncate(d) == d,
        _ => false,
    };

    private static bool Same(JsonElement a, JsonElement b) => (a.ValueKind, b.ValueKind) switch
    {
        (JsonValueKind.String, JsonValueKind.String) => a.GetString() == b.GetString(),
        (JsonValueKind.Number, JsonValueKind.Number) => a.TryGetDecimal(out var x) && b.TryGetDecimal(out var y) && x == y,
        _ => false,
    };
}
