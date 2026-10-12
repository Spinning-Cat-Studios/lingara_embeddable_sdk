package com.getlingara.embed.java.conformance;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.util.Iterator;
import java.util.Map;

/**
 * How a {@code poll_until} matches an event (ADR 1.10.26y D12): the event must contain the case's
 * {@code match}, compared after dropping nulls.
 */
final class Matching {
  private Matching() {}

  static JsonNode withoutNulls(JsonNode value) {
    if (value.isObject()) {
      ObjectNode out = Harness.JSON.createObjectNode();
      value
          .properties()
          .forEach(
              e -> {
                if (!e.getValue().isNull()) {
                  out.set(e.getKey(), withoutNulls(e.getValue()));
                }
              });
      return out;
    }
    if (value.isArray()) {
      ArrayNode out = Harness.JSON.createArrayNode();
      value.forEach(item -> out.add(withoutNulls(item)));
      return out;
    }
    return value;
  }

  static boolean contains(JsonNode actual, JsonNode wanted) {
    if (wanted == null || wanted.isNull() || wanted.isMissingNode()) {
      return true;
    }
    if (actual.isObject() && wanted.isObject()) {
      return fieldsMatch(actual, wanted);
    }
    if (actual.isArray() && wanted.isArray()) {
      return actual.size() == wanted.size() && elementsMatch(actual, wanted);
    }
    return actual.equals(wanted);
  }

  private static boolean fieldsMatch(JsonNode actual, JsonNode wanted) {
    for (Map.Entry<String, JsonNode> entry : wanted.properties()) {
      JsonNode got = actual.get(entry.getKey());
      if (!entry.getValue().isNull() && (got == null || !contains(got, entry.getValue()))) {
        return false;
      }
    }
    return true;
  }

  private static boolean elementsMatch(JsonNode actual, JsonNode wanted) {
    Iterator<JsonNode> got = actual.elements();
    Iterator<JsonNode> want = wanted.elements();
    while (got.hasNext()) {
      if (!contains(got.next(), want.next())) {
        return false;
      }
    }
    return true;
  }
}
