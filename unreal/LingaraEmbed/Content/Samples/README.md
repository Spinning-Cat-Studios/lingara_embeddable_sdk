# The sample (not yet authored)

The sample is five editor assets, all Blueprint (ADR 1.10.26ac D8). Only
the Unreal Editor can write them, and none has been authored yet: the first
engine session authors them in the oldest supported engine version (the
first line of `unreal/engine-versions.txt`) before its smoke, because the
smoke packages them. Together they stay under 2 MB, as plain binary files
with no Git LFS.

| Asset | What it does |
|---|---|
| `L_LingaraSample` | One room and one NPC. The smoke project's default map. |
| `BP_LingaraSampleNpc` | On interact, calls **Dialogue Open** once, then **Lingara Say**, showing the streamed fragments in `WBP_LingaraDialogue`. A two-level picker calls **Dialogue Set Level**. |
| `WBP_LingaraDialogue` | The NPC's reply, appended fragment by fragment. |
| `WBP_LingaraPractice` | A practice board: **Practice Start** with a `Plan` source and `Report` off (a demo plan), then **Practice Choose** or **Practice Answer**, then **Practice Next**. |
| `BP_LingaraSampleGameMode` | On **On Token Needed**, runs **Fetch Token From URL** against the URL typed into the sample widget, by default `tools/demo-token-server` on `http://127.0.0.1:8787/token`. |

C++ usage is the snippet's job: `snippets/player/unreal/`.
