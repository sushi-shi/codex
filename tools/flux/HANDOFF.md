# Resume checkpoint

The user requested a pause after completing the String-model change, recording this handoff, and cleaning build artifacts. Resume on `spec/flux-parser-slices`, stacked on `spec/flux-parser-positions` (draft PR #18).

The matching Flux branch is `feat/string-slice-drain` ([draft PR #30](https://github.com/sushi-shi/flux/pull/30)). Its verified compiler/model commit is `ef5241b9501554f1f4d95a676bdb0e3d4495f898`. Start with [Flux's current handoff](https://github.com/sushi-shi/flux/blob/feat/string-slice-drain/handoff/CURRENT.md) for the implementation, rebuild commands and next steps. The native macro dependency remains at `d2b7ea21d4177217e10e93ebff162bbec9da9f19`.

This Codex change verifies that `longest_suffix_prefix_len` returns a length whose suffix starts at a UTF-8 boundary in the input. It adds one contract and a caller-purpose comment; runtime statements are unchanged. Removing the `ends_with` guard is rejected by the checker. All 30 native parser tests pass, and the supporting Flux revision passes 1,174 regressions (7 ignored).

The stronger String models expose four remaining obligations: the slice and drain at `take` in `drain_visible_to_suffix_match`, and the range/boundary of the opening-tag drain endpoint in `push_str`. These are ordinary refinement failures. The final run contains no compiler crash or timeout. The closing-tag endpoint and active-tag suffix split now verify under the models.

[parser-slices.json](parser-slices.json) records exact revisions, scope, timings and assumptions. [parser-slices-evidence.json.gz](parser-slices-evidence.json.gz) contains the reports, mutation evidence and validation logs. The broader parser sweep takes 139.790 seconds, including 130.937 seconds of solver work; reducing that cost remains part of the next work.

Continue by propagating the suffix predicate through `Map`/`max`, stating the drain helper's valid-split requirement, and relating the selected tag index to its delimiter's end. Exact drain effects still need a model that distinguishes normal Drop from forgetting the iterator. Do not weaken the new boundary models to recover an earlier passing status.

After cleanup, the retained checkouts are `/home/sheep/Projects/codex-flux` and `/home/sheep/Projects/flux`. Build targets are removed; rebuilding is expected. The Codex checkout has its own Git objects and does not depend on the removed temporary clones. Read `AGENTS.md`, use two Cargo jobs, use `just fmt` and `just test`, and keep the full Codex verification goal intact.
