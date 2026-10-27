# Immutable sources and recovery

Authorized root only:
`/Users/almat/Desktop/code/Tezbar APP/rust-extension-ports/modify-hash`.
This task never uses an additional worktree or subagent and never retries Pi.
The supplied outer directory was not used as a Git checkout.

The newly created partial clone had HEAD
`3fdbb4822b6d9892bfbb685094a3e8a7b69270f0`, an empty index/incomplete checkout,
and no user or agent changes. A fresh `git fetch origin` succeeded. A bounded
HTTP/1.1 Git restore timed out on lazy blob retrieval. Its own checkout transfer
ended; no other repository process was terminated. The interrupted checkout's
stale index lock was moved into ignored `.qa/interrupted-checkout-index.lock`.

All 23 HEAD tree blobs were recovered from immutable
`https://raw.githubusercontent.com/almatkai/modify-hash/3fdbb4822b6d9892bfbb685094a3e8a7b69270f0/<path>`
with bounded requests. For every byte buffer, Git's SHA1
`blob <length>\0<bytes>` matched the tree's object ID before `git hash-object -w`
and restoring this exact clone from HEAD. The resulting V1 tree was clean,
including the unchanged lock, before creating `feat/native-v2-rust` from fresh
`origin/main` at that SHA. No delete/reclone or reset of any other checkout occurred.
`evidence/objects.json` records every original blob ID and SHA256.

SDK references use **git show only**, not dirty working files, from the read-only
sibling `tezbar-extension-sdk-rust` at
`e955911f0bf5b558f9d3eb5183df0adc95c22af2` (crate 0.1.1). Root docs
`SDK_API.md`, `PROTOCOL.md`, `HOST_INTEGRATION.md`, actual runner/context source,
LICENSE and NOTICE are copied under evidence/sdk. Cargo uses an immutable
HTTPS dependency rather than a local path; fresh isolated locked builds prove
acquisition without a sibling dependency.

Host references use **git show only** at
`ea100cee3f69711de12485ba061950c5fc65258f` in the read-only Tezbar checkout.
The external process guide, schema, renderer Form/List/Detail/outer surfaces,
reverse selection producer and platform files are copied and hashed under
`evidence/host`. Host working files are dirty and unrelated and were not read,
built, stashed, checked out or mutated. The standalone host prerequisite patch
is generated in this clone only and remains unapplied.

Icon bytes were fetched into memory from exact upstream
`raycast/extensions@6734d85992290066fd31a7f5bee3cca3d3545a1a`, path
`extensions/modify-hash/assets/hasher.png`. They equal the own V1 icon byte for
byte. SHA256:
`df59209ea6d50585d5fca4214ae616ce98c3bfb824b0b5fc79ab2f8063c94fc5`.
Only `v2/resources/hasher.png` is added; V1 assets are untouched.

Canonical XERJ prefix is `ax-8a09adc077e6`, derived from this root's SHA256.
The prescribed local server was reachable. Initial autoindex timed out and
own-prefix search failed HTTP 400 with strict unknown `defs`/`name` fields.
No alternate prefix, server restart, schema change, or settings bypass was used.
Full own source reads, exact rg, immutable Git objects and hashes supplied review
coverage. The indexing attempts and remaining V2 coverage limitation
are recorded in VALIDATION.md; no completeness claim is made for XERJ.

All source-library tests use the own V1 locked versions: file-type 21.3.4 and
AdmZip 0.5.18. The actual TSX and complete locked dependency closure are bundled
inside a closed VM with path/fd/sync/promises/callback/FileHandle/stream guards
installed before any source callback is invoked. Synthetic parent directory
facts let AdmZip's mkdir walker operate without probing real ancestors outside
QA. There are no actual user files, clipboard/selection, keychain, Downloads,
profile, product APIs or network scans in test fixtures.

One isolation deviation occurred during initial dependency inspection: early
Cargo fetch/check/test calls used Cargo's existing default cache, before QA
isolation was installed. No user content or host/profile files were involved.
The final gates use synthetic HOME/TMP/cache and isolated Cargo homes/targets
under `.qa`, and the fresh HTTPS build starts with an empty own Cargo home.
Existing cache entries are not deleted or edited as a cleanup operation.
Python's initial import bytecode was moved into `.qa`; later QA imports disable
bytecode output. This exception is disclosed rather than claiming every early
tool cache write was isolated.

Round 2 changes only the standalone adapter's draft/selection ownership, the
separate generated prerequisite patch, regression scripts and review documents/
evidence. Pinned host and SDK evidence copies remain byte-identical. Renderer
regressions transpile the actual patched Form and surface components, execute
their hook/callback boundaries in a closed synthetic adapter, then connect those
callbacks over JSONL to the extracted production executable. This is component/
protocol execution, not real React DOM mounting or desktop GUI acceptance. Source
hashes identify the original immutable host objects; patched-copy hashes and the
adapter/executable hashes are separately identified in renderer evidence.
