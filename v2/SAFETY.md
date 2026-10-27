# Filesystem and lifecycle review

## Authority and paths

The process uses real native filesystem calls. It is not an OS sandbox. Only
explicitly submitted absolute paths and their ordinary descendants are visited.
No HOME, Downloads, clipboard, profile, network scan, or live product API is
consulted. Form validation does not inspect the filesystem. Selection is a
separate explicit, read-only reverse request and never executes a batch.

`safe_fs::parent` acquires only a filesystem root with ambient authority, then
opens every normal path component relative to pinned directory handles using
no-follow operations. The final source opens no-follow, checks ordinary type,
identity, length and modification time against the collected metadata, and
rechecks the held handle and directory entry before replacement. Unix file opens
are nonblocking so a racing FIFO substitution cannot block the process before
its type check. Windows uses cap-std's handle metadata and rejects reparse
attributes. Symlinks, junction/reparse points, devices, sockets, FIFOs, parent
traversal, and filesystem-root selections fail explicitly. Spaces, Unicode,
newlines, punctuation, arbitrary filename extensions, files and directories
are supported; paths are not shell-interpolated. Nested non-UTF-8 filenames are
supported for hash actions (display is lossy); ZIP names must be UTF-8.

All replacements stage in the **destination directory**, independent of the
SDK's temp directory or its volume. Unix stages are created private (file mode 0600, tree mode 0700). Source
permissions are copied before atomic replacement. ZIP trees also stage beside their destination. New outputs publish
with no-replace rename: rustix `NOREPLACE` on Unix, `FILE_RENAME_INFO` relative to
the pinned destination handle with ReplaceIfExists=false on Windows. Known
collisions advance V1's numbered suffix. Symlink output collisions are never
followed. Input mutation rechecks identity immediately before rename.

There is no portable atomic filesystem compare-and-swap of a pathname against
an inode. Another writer can replace an entry between the last identity check
and rename. Parent handles and no-follow checks prevent following a symlink out
of scope; they do not serialize adversarial competing rename/content writers.
The process serializes its own filesystem batches across sessions; concurrent
external editing is unsupported and detected changes are errors. A selected
ancestor moved by another process can still be addressed through its pinned
handle. Close external writers before submitting important files.

Atomic replacement changes the selected directory entry's inode. Unlike V1's
in-place append/truncate, other hard-link aliases retain their old inode; aliases
explicitly selected twice in a batch are deduplicated by original identity.
Payload and mode are preserved; ownership, ACLs, xattrs, resource forks, and
original timestamps are not promised. These metadata and hard-link adaptations
are deliberate and are not described as exact V1 filesystem metadata parity.
File sync is attempted before publication; directory fsync, crash recovery, and
exactly-once or power-loss durability are not promised. Hard crashes can leave
random `.modify-hash-<uuid>.tmp` stages. They are not automatically deleted on a
later run because the new process cannot prove ownership of arbitrary leftovers.

## Archives

Compression preserves ordinary selected content and uses Stored/Deflate portable
ZIPs. Source paths with conflicting portable member names fail, rather than
letting AdmZip replace an earlier member. Extraction supports Stored and Deflate,
ZIP64 local and central metadata, and validates content CRC and expanded sizes.
Encrypted entries are explicit errors. A bounded raw central-directory preflight
runs before `ZipArchive` construction because zip 2.x otherwise collapses exact
duplicate names. Declared counts, directory range and decoded index count must
agree. Multi-disk archives, nonstandard central-directory records, and archives
with bytes after the EOCD comment fail explicitly. These are documented archive
compatibility limits, not successful no-op extraction.

Every member is checked before creation: reject absolute/drive/UNC paths, `..`,
empty or dot components, backslashes, colons, NUL/control characters, portable
Windows device names, trailing dot/space components, symlink/special Unix modes,
and duplicate or case-folded conflicting files/directories across all input
archives. File-as-parent conflicts fail. Directory entries cannot redirect writes.
Files are created exclusively relative to staged directory handles; implicit
parents are opened no-follow. OS collision checks remain authoritative for
filesystem-specific Unicode normalization. Any failure drops the entire new
staged tree, leaves existing outputs intact, and reports failure. V1's overwrite
of conflicting entries is intentionally replaced by this fail-closed policy.

ZIP extraction uses V1's first-dot output naming. A leading-dot archive whose
V1 name becomes empty receives an explicit naming error instead of selecting
its parent directory. Password defaults and rejection match the own V1 source;
no fake encryption, shell zipper, password logging or success stub is present.

## Explicit bounds

- 128 selected paths; absolute input path length at most 32768 bytes.
- Complete form at most 128 KiB; password at most 1024 UTF-8 bytes.
- 4096 walked nodes, or archive explicit/implicit paths; maximum depth 64.
- 8 GiB per source/expanded member, 16 GiB total selected/expanded content.
- ZIP expansion ratio at most `1000 × compressed bytes + 1 MiB` per member.
- 16 MiB raw central directory; 1 MiB EOCD/ZIP64 tail probe; 4096-byte ZIP names.
- 16 MiB content probe for media category detection; large valid container
  metadata beyond that probe yields an explicit error, not an extension guess.
- V1 media parser bounds are retained where translated: 256 reentries, 512 PNG
  chunks/ASF objects/TIFF tags, bounded EBML children and payloads. Category
  inference follows the locked content rules, including audio/image BMFF brands,
  Ogg codec categories and the source's default zero MPEG offset tolerance.
  Malformed containers can produce clearer explicit errors than V1's skip.
- 256 KiB streaming chunks, at most one process-owned filesystem batch; SDK
  protocol frames 16 MiB, emitted views 12 MiB, preflight report estimate 4 MiB.
  Oversized diagnostics are replaced by an explicit diagnostic-size error,
  rather than silently truncated into a misleading result.
- SDK session capacity 128. There is no polling, detached timer/task, actor thread,
  background lease, media preview or unbounded output list in domain code.

Bounds fail explicitly; no list of selected items, media bytes or archive member
is silently truncated. Filesystem and native platform failures remain errors.

## Action ownership and recovery

All seven editable fields travel in a complete snapshot plus monotonic draft
revision. The separate unapplied renderer patch increments revision on **every**
field change, including A/B/A changes and invalid intermediate text, and keeps
local drafts authoritative over older and equal-revision server echoes. Before
selection dispatch it captures a pending request/revision owner, invalidated
synchronously by every edit. The process returns an effect-free selection
candidate without changing fields or taking a client revision. Only an unchanged
request owner can automatically admit it through a one-use acceptance action
with the next client revision, expected paths and unchanged preferences. This
prevents the serialized SDK reverse-RPC wait from hiding queued edits. Repeated
selection, invalid intermediates, late candidates and stale responses do not
restore ownership. Selection errors/cancellation preserve an editable draft and
fresh selection/submit tokens; old effect tokens remain consumed. The extension fences
submit against its latest full snapshot and consumes a matching effect action
ID before validation or await, in adapter-owned memory outside SDK staged Value
rollback. Stale, unknown, malformed-submit, queued or already-consumed IDs cannot
replay effects. Field changes, search/load-more and refresh are effect free.

The SDK serializes operations within a session. The domain process rejects an
additional filesystem batch while another session owns the filesystem lock.
It never leaves an old batch queued behind another session's filesystem action.
No reverse RPC is needed to mutate submitted files. Explicit selection replies
must be an array of absolute strings, at most 128; null, boolean, objects,
relative paths and nonstring members fail. No clipboard, paste, shell, settings,
window visibility or hidden focus service is used, so no ACK contract is guessed.

Each file commit is followed synchronously by recording its result, before the
next cancellation checkpoint. MD5 before/after is derived during the same pass,
never by rereading an acknowledged mutation; disabling the MD5 log skips hash
computation for hash actions. Restore of an enabled media file with no marker
still reports equal MD5 values when logging is enabled, matching V1. ZIP groups
publish once, then record the final output path. Refresh/retry returns frozen
results, with no recompression or filesystem replay. Failure records distinguish
committed, unchanged, skipped and failed items. Cancellation stops at bounded
checkpoints, and stage guards clean up when futures drop; already committed
items remain recorded. Interrupted sessions report that remaining items were
not automatically retried. A deliberate **Start a New Batch** action creates a
new intent, not a retry token. A process/OS crash or lost process memory can make
outcomes ambiguous: inspect files/outputs before a new submission.

Open has no awaits or domain registry allocation. SDK canceled-open cleanup can
therefore omit the dispose hook without stranding adapter state. Production pipe
QA demonstrates 140 canceled opens without dispose, capacity afterward, canceled
batches, committed-result recovery, EOF cleanup and no background continuation.
The production binary's own stdout remains protocol-only.
