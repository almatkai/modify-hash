# Parent review validation

Selected root:
`/Users/almat/Desktop/code/Tezbar APP/rust-extension-ports/modify-hash`.
Branch `feat/native-v2-rust`; source/base/fresh origin/main
`3fdbb4822b6d9892bfbb685094a3e8a7b69270f0`. No commits were created.
Final status is only ` M README.md` and `?? v2/`; the index is empty. The root
README has one short discovery link; every other historical tracked byte is
verified against the original immutable object hashes. No additional checkout,
host, SDK, sibling, profile, ledger, catalog or acquisition file is changed.

## Required gates

All seven `scripts/verify.py` steps passed; inspect `evidence/verify-results.json`.
The final `scripts/finalize.py` package and extracted executable suites also passed.

| Gate | Evidence and result |
|---|---|
| Original unchanged V1 | `npm ci --ignore-scripts`, 1 original smoke test, typecheck pass in owned sourcecopy; lock unchanged. No original test/typecheck failure to retain. |
| Actual own TSX oracle | 6 command checks covering modify, restore, unrecognized suffix, default media filtering, real ZIP compression/extraction and collision naming. |
| Locked media library oracle | 78 content fixtures from actual file-type 21.3.4, compared against the extracted Rust production executable; all match category-driven mutation behavior. Includes BMFF image/audio/video brands, Ogg, EBML, ASF, TIFF/raw, PNG, JPEG2000, ID3 and BOM cases. Not proof of all possible corrupt or future formats. |
| Original locked archive oracle | Actual AdmZip 0.5.18 executes behind the preinstalled guarded VM filesystem boundary. Root dependency ranges and lock remain unchanged. |
| Rust stable | 7 tests, fmt check, strict all-target Clippy pass on rustc 1.98.0. |
| Rust MSRV | Same 7 tests and strict all-target Clippy pass on rustc 1.85.0. |
| Windows GNU | All-target check and strict Clippy pass. Native linking probe fails: missing `x86_64-w64-mingw32-dlltool`. No compiler/tool installation, executable or Windows OS success claim. |
| Pinned host parsing/producers | 13 checks using actual Form/List/Detail helpers and full transpiled Form/surface component callbacks with deterministic hooks; 8 additional cases connect those producers/reconciliation to the extracted production SDK process (21 total). Covers synchronous pending ownership, A/B/A, invalid→valid, preferences, repeated selection, late/equal-revision replies, candidate replay, errors/cancellation and latest explicit submit. Literal Finder script runs only against a synthetic Finder. No real React DOM, full host build or GUI run. |
| Schema | Unchanged exact pinned schema accepts the source manifest; 16 negative cases reject. The standalone evaluator implements every assertion keyword used by that schema, fails closed on other keywords, and does not claim general metaschema certification. |
| Production SDK JSONL | Extracted production executable: 174 assertions across 64 named cases. Real hello/initialize/session/action/dispose/shutdown pipes. 140 canceled opens in 140 attempts without dispose; new session succeeds afterward. |
| Extended production | Extracted executable: 97 assertions across 15 named groups, including 78 media oracle comparisons, recursive/overlapping inputs, punctuation, read-only destination, CRC, archive conflicts, ZIP64 local headers, password rejection, concurrency, EOF drop cleanup and >16 MiB inbound frame rejection. |
| Cancellation/recovery | Cancellation left 3 committed file in this run; all file snapshots were exact old or exact committed bytes, and recorded commit count matched actual mutations. The number is timing-dependent, not a guarantee. Reusing the token made no further mutation. |
| Output safety | Traversal, absolute/backslash/device names, duplicate/case/prefix conflicts, ZIP symlink entries and ratio-bomb inputs reject. Symlink leaf and ancestor do not redirect writes. Failed extraction leaves no published partial output. Staged files disappear after cancellation/EOF. |
| SDK temp volume independence | Synthetic SDK temp path is never created or used; implementation stages through destination directory handles. No actual cross-volume GUI test was run. |
| Fresh HTTPS build | New empty owned Cargo home/target; `cargo build --release --locked` acquires SDK over immutable HTTPS and passes. `evidence/fresh-build.json` pins the exact source hashes and executable. No sibling path dependency. |
| Licenses | 84 complete-graph/QA-source package records; authentic license/notice/AUTHORS evidence copied. Zero missing-text evidence records after including r-efi's actual AUTHORS grant. Not a legal review. |
| Historical bytes | All original V1 objects except the appended README discovery link retain exact SHA256; icon additionally verified against exact pinned Raycast bytes. |
| Artifact scope | No node_modules, target, .qa, dist, Python bytecode or ZIP is Git-visible or staged. No root CI workflow is activated. |

Tool versions are in `evidence/tool-versions.json`; copied detailed results,
fixture originals, immutable object/library hashes and package inventory are in
`evidence/`. All runnable outputs and synthetic archive/source fixtures remain
owned and ignored under `.qa/` or `dist/`.

## Round 2 selection regression

The reproduced defect was a server-side selection completion incrementing the
same ordinal used by an already queued client field snapshot. Equal ordinals
caused the queued paths/preferences to be ignored and React reconciliation to
replace the visible draft. The adapter now returns a one-use selection candidate;
the renderer captures ownership synchronously before dispatch and automatically
accepts only if no field event has occurred since. Only the client increments
its revision. Acceptance is effect-free, changes only paths, and must match the
candidate's expected complete snapshot before a new submit token is issued.

Production-pipe regressions hold a real reverse RPC while enqueueing edits,
including the original revision-1 paths/MD5-loss case, A/B/A, invalid→valid and
preference-only changes. Additional cases cover one-use/repeated admission,
old candidate/submit replay, preference-smuggling rejection, empty-selection
recovery, actual protocol cancellation with a late reply,
reverse errors and recovery without reopening. The complete patched Form and
surface callbacks run under synthetic hooks, and eight JSONL integration cases
execute those actual producers against the extracted production executable.
No selection, field update or candidate acceptance mutates any fixture; only
valid latest explicit submit changes the visible selected file. The deterministic
hook adapter does not claim a browser/React DOM or full desktop GUI test.

## Development artifact

`dist/modify-hash-v2-0.2.0-aarch64-apple-darwin-dev.zip`:

- ZIP SHA256 `851d255ba1f2505e3b4e47c706b3097b2abb698d19c0700eded7305b888bd256`.
- Executable SHA256 `c73a7175056dd9202cf5edc6df6f62820c9d3fc61cb86174ce8716d461a277ed`.
- 186 exact closed members, including extension.json; its internal files map
  inventories all 185 other members. Every extracted file matches the report.
- Two independently constructed ZIP buffers are byte-identical, with sorted
  members, fixed 1980 timestamp, explicit regular-file modes and Stored encoding.
- Extracted protocol identity/version/commands verified before production suites.
- Source and packaged manifests retain `artifacts: {}`. No fake download pin,
  catalog edit, acquisition entry, installed version, or release is created.

Exact icon SHA256:
`df59209ea6d50585d5fca4214ae616ce98c3bfb824b0b5fc79ab2f8063c94fc5`.

## Honest remaining limitations

- Both platform verification records remain **not-run**. Real macOS and Windows
  GUI, Finder/Explorer/focus selection, Windows handle/reparse execution, Windows
  MSVC artifact/signing and desktop installation are unrun.
- The three-file host prerequisite patch remains **unapplied**. The process's
  strict effect submission needs complete versioned snapshots. Manual absolute
  JSON path entry is provided; a native chooser is not implemented in the process.
- Catalog/acquisition/host/profile/SDK changes and release/install/PR/push/commit
  remain parent-owned and unapplied.
- `SAFETY.md` documents all explicit caps, archive restrictions, inode/hard-link/
  metadata adaptations, competing-writer limits and lack of crash durability.
  No exactly-once claim or replay after acknowledged effects is made.
- XERJ coverage remains incomplete. The earlier own-prefix autoindex hit
  the real RSS circuit breaker (`rss=791MB`, watermark 768MB) and was stopped at
  35 seconds. Round 2 autoindex finished at exit 3: the docs mapping was refused
  (keyword→text type conflict), 141 files refused and all 41 detected source
  files unindexed. The post-change attempt aborted at exit 1 because the frozen
  dataset cannot accept the updated FILE_HASHES.json field schema (0.3 seconds).
  Own-prefix search returns strict unknown-field HTTP 400.
  No alternate prefix, server restart or schema/settings bypass was used.
  Review coverage is supplied by full reads, rg, Git objects and hashes.
- Early Cargo dependency inspection used the existing default cache before
  isolation was configured. Final gates and fresh builds are isolated. This
  exception is recorded in PROVENANCE.md; shared cache state is not erased.

`FILE_HASHES.json` inventories every Git-visible V2 file and the root README,
excluding itself and historical V1. It hashes working bytes, not an assumed
staged snapshot. Parent: rerun `python3 v2/scripts/finalize.py --hash-only` after
staging, stage the resulting index, and compare staged blobs before committing.
Full verify/finalize may be repeated without commit/push/install actions.
