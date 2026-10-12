#!/usr/bin/env bash
# release-toml.sh — the one reader of tools/release/release.toml (ADR 1.10.26ag D3).
#
#   release-toml.sh matrix                 the natives job's legs: {"include":[{rid, runner, container}]}
#   release-toml.sh rids                   every platform rid, one per line
#   release-toml.sh container <rid>        the platform's build container, or nothing
#   release-toml.sh natives <rid>          one line per [[native]] that applies: its make arguments
#   release-toml.sh outs <rid>             one line per [[native]] that applies: the directory it writes
#   release-toml.sh package <id> <rid>     the paths package <id> must hold for <rid>, placeholders expanded
#   release-toml.sh package-rids <id>      the rids package <id> carries (its native's)
#   release-toml.sh package-native <id> <rid>  the directory holding the bytes <id> packs for <rid>
#   release-toml.sh per-engine <id>        `true` when <id> is a bundle of per-engine zips
#
# Shell has no TOML parser, so python3's tomllib reads the file: python3 is on
# every runner image and version-check.sh needs it anyway. RELEASE_TOML points
# it at a fixture. An unknown id or rid is exit 2, never an empty answer.
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
toml=${RELEASE_TOML:-$here/release.toml}
py=$(command -v python3 || command -v python || true)
[ -n "$py" ] || { echo "release-toml.sh: python3 is required" >&2; exit 2; }
[ $# -ge 1 ] || { sed -n '4,13p' "$0" >&2; exit 2; }

exec "$py" - "$toml" "$@" <<'PY'
import json, sys, tomllib

# Windows' text-mode stdout writes \r\n, and every caller is bash `read`,
# which keeps the \r: `make ffi-dist-win-x64\r` has no rule. Its console
# encoding is cp1252, so UTF-8 too, as version-check.sh does.
sys.stdout.reconfigure(newline="\n", encoding="utf-8")
sys.stderr.reconfigure(encoding="utf-8")
path, verb, args = sys.argv[1], sys.argv[2], sys.argv[3:]
with open(path, "rb") as f:
    doc = tomllib.load(f)
rids = [p["rid"] for p in doc["platform"]]

def fail(msg):
    print(f"release-toml.sh: {msg}", file=sys.stderr)
    sys.exit(2)

def need(n):
    if len(args) != n:
        fail(f"{verb} takes {n} argument(s)")

def rid_arg(rid):
    if rid not in rids:
        fail(f"unknown rid {rid!r}; one of {' '.join(rids)}")
    return rid

def natives_for(rid):
    return [n for n in doc["native"] if rid in n.get("rids", rids)]

def native(name):
    return next(n for n in doc["native"] if n["name"] == name)

def package(pid):
    for p in doc["package"]:
        if p["id"] == pid:
            return p
    fail(f"no [[package]] {pid!r}")

def expand(template, rid):
    out = [template.replace("{rid}", rid)]
    if "{ue_platform}" in template:
        out = [t.replace("{ue_platform}", doc["ue_platform"][rid]) for t in out]
    if "{jna_prefix}" in template:
        out = [t.replace("{jna_prefix}", j) for t in out for j in doc["jna_prefix"][rid]]
    return out

if verb == "matrix":
    need(0)
    legs = [{"rid": p["rid"], "runner": p["runner"], "container": p.get("container", "")} for p in doc["platform"]]
    print(json.dumps({"include": legs}, separators=(",", ":")))
elif verb == "rids":
    need(0)
    print("\n".join(rids))
elif verb == "container":
    need(1)
    rid = rid_arg(args[0])
    print(next(p for p in doc["platform"] if p["rid"] == rid).get("container", ""))
elif verb == "natives":
    need(1)
    rid = rid_arg(args[0])
    for n in natives_for(rid):
        print(n.get("make", n["name"] + "-dist RID={rid}").replace("{rid}", rid))
elif verb == "outs":
    need(1)
    rid = rid_arg(args[0])
    for n in natives_for(rid):
        print(n["out"].replace("{rid}", rid))
elif verb == "package":
    need(2)
    p, rid = package(args[0]), rid_arg(args[1])
    if rid not in native(p["native"]).get("rids", rids):
        fail(f"{args[0]} carries no {rid}")
    print("\n".join(expand(p["path"], rid)))
elif verb == "package-rids":
    need(1)
    print("\n".join(native(package(args[0])["native"]).get("rids", rids)))
elif verb == "package-native":
    need(2)
    rid = rid_arg(args[1])
    print(native(package(args[0])["native"])["out"].replace("{rid}", rid))
elif verb == "per-engine":
    need(1)
    print("true" if package(args[0]).get("per_engine_zip") else "false")
else:
    fail(f"unknown subcommand {verb!r}")
PY
