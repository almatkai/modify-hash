# Modify Hash — standalone Rust V2

Review-only development port of **all four own portable V1 commands**. Nothing
is installed, registered in Tauri, published, or added to the host catalog.
`extension.json` is schema 3, external process, protocol 1, SDK 0.1.1 pinned over
HTTPS to `e955911f0bf5b558f9d3eb5183df0adc95c22af2`. Rust MSRV is 1.85.

V1 source: `3fdbb4822b6d9892bfbb685094a3e8a7b69270f0`. Only the root README
has a discovery link; all other historical V1 tracked bytes, lock, license,
metadata, and icon remain unchanged. `evidence/objects.json` pins their hashes
and the exact SDK and host objects used. `PROVENANCE.md` describes recovery.

## Behavior

- `modifyHash`: recurse through explicitly submitted ordinary files/directories,
  detect media from contents, and append exactly `#$1024$#`, including repeated
  deliberate appends. No extension-based media filter is imposed.
- `restoreHash`: remove exactly one trailing marker. Preserve every byte when
  the exact suffix is absent. It does not search for interior markers, guess
  original lengths, or remove unrecognized suffixes.
- `zipCompress`: real Stored/Deflate-compatible ZIP creation using selected
  basenames, preserving nested and empty directories. One input gives
  `<input>.zip`; multiple inputs give `<first stem> (N files).zip` beside the
  first input. Existing outputs get ` 2`, ` 3`, etc., before the extension.
- `zipExtract`: case-insensitive `.zip` selection filter; combine the selected
  archives in one output directory. Preserve V1's first-dot naming rule and
  numbered collision behavior. Conflicting members fail the whole staged
  extraction instead of V1's silent overwrite.

Defaults are copied from the own package: MD5 log **true**, video **true**, audio
**false**, image **false**, password enabled **false**, password `1024`.
Enabling a nonempty trimmed password fails explicitly, as the current own V1
source does. Neither implementation provides encryption. Password text is never
logged. Copy/paste/browser/hide actions do not exist in the own source and are
not invented here. The sole reverse service is explicit `selection.read` with
`kind: files`; the reply must be an absolute string array.

Open displays a form, without reading selection or mutating files. Enter a JSON
array of absolute paths (including paths with spaces, quotes, or newlines), or
explicitly request Finder/Explorer selection. A returned selection is accepted
automatically only if the requesting draft has remained untouched; edits made
while selection is pending always win, including A/B/A and invalid text. Selection
never increments the client revision or changes files. Every preference can be edited for
this batch. Submit is the only filesystem action. A native chooser is not
implemented in the process; the form uses TextArea rather than the host's
browser FilePicker, which can return relative names. Native chooser integration
would be separate host work and is not claimed here.

## Host prerequisites — unapplied

See [INTEGRATION.md](INTEGRATION.md) and `host-prerequisites.patch`, against
host `ea100cee3f69711de12485ba061950c5fc65258f`. The pinned renderer does not
send complete versioned field snapshots; the pinned selection producer silently
limits selection and filters directories. The process fails closed for unsafe
submit contracts. The patch is tested through pinned Form/List/Detail parsing,
reconciliation, actual extracted change/submit producers, and the literal Finder
script using a synthetic Finder. It has not been applied to the host. PowerShell
and full desktop GUI acceptance are unrun. Both manifest platform verification
states deliberately remain `not-run`.

## Safety and differences

Read [SAFETY.md](SAFETY.md) before reviewing filesystem changes. It states the
explicit resource limits, platform bounds, metadata adaptations, cancellation
and collision policy, and lack of crash durability. This is not an OS sandbox.
No selection is executed on open/refresh/field changes. Frozen action IDs are
consumed before awaits outside SDK staged rollback; completed results are kept
in memory for refresh and retries, without rereading or replaying effects.

## Reproduce

From this repository root, with existing Rust stable/1.85 toolchains, Node/npm,
and Python 3.9+ (no global tools are installed):

```sh
python3 v2/scripts/verify.py
python3 v2/scripts/finalize.py
```

All outputs, fixture paths, synthetic HOME/TMP/cache, source copy, Cargo homes,
archives, and extracted executables live under ignored `v2/.qa/` or `v2/dist/`.
`verify.py` reports actual failures and performs a fresh isolated locked HTTPS
SDK build. A Windows GNU linker probe is informational; Windows GNU check and
strict Clippy remain required, and no Windows release is claimed. Source V1
`npm ci --ignore-scripts` uses its unchanged lock in a disposable copy.

`finalize.py` packages only a verified ARM64 executable, authentic notices and
licenses, the exact icon, and a closed hash inventory. ZIP member order, times,
modes and bytes are deterministic. It extracts and hashes every member, runs the
production executable through JSONL pipes, and writes `FILE_HASHES.json` for all
Git-visible V2 files plus the README discovery link, excluding itself and
historical V1. The parent must rerun it after staging and recheck the index.

Review evidence and exact gate counts are in [VALIDATION.md](VALIDATION.md).
No commits, push, PR, release, install, host/profile/catalog/acquisition/SDK/sibling
writes, or clone deletion are performed. The final index is left empty.
