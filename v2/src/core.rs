use crate::{
    media::{self, Media},
    safe_fs::{self as fs, Node, Result, Stage, Walk, MAX_FILE, MAX_ITEMS, MAX_TOTAL},
};
use cap_fs_ext::DirExt;
use cap_std::fs::Dir;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};
use tezbar_extension_sdk::ExtensionContext;
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};
pub const MARKER: &[u8] = b"#$1024$#";
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[allow(non_snake_case)]
pub struct Draft {
    pub paths: Vec<String>,
    pub showMD5Log: bool,
    pub enableVideo: bool,
    pub enableAudio: bool,
    pub enableImage: bool,
    pub enableZipPassword: bool,
    pub zipPassword: String,
}
impl Draft {
    pub fn defaults(p: &serde_json::Value) -> Self {
        Self {
            paths: vec![],
            showMD5Log: p["showMD5Log"].as_bool().unwrap_or(true),
            enableVideo: p["enableVideo"].as_bool().unwrap_or(true),
            enableAudio: p["enableAudio"].as_bool().unwrap_or(false),
            enableImage: p["enableImage"].as_bool().unwrap_or(false),
            enableZipPassword: p["enableZipPassword"].as_bool().unwrap_or(false),
            zipPassword: p["zipPassword"].as_str().unwrap_or("1024").into(),
        }
    }
    pub fn validate(&self, command: &str) -> Result<()> {
        if self.paths.is_empty() || self.paths.len() > 128 {
            return Err("Select 1–128 absolute files or directories".into());
        }
        if self.zipPassword.len() > 1024 {
            return Err("Password field exceeds 1024 bytes".into());
        }
        for p in &self.paths {
            fs::absolute(Path::new(p))?;
        }
        if matches!(command, "modifyHash" | "restoreHash")
            && !self.enableVideo
            && !self.enableAudio
            && !self.enableImage
        {
            return Err("Enable at least one media type".into());
        }
        if matches!(command, "zipCompress" | "zipExtract")
            && self.enableZipPassword
            && !self.zipPassword.trim().is_empty()
        {
            return Err(
                "Encrypted zip archives are not supported by the portable V1 or V2 commands".into(),
            );
        }
        Ok(())
    }
    fn enabled(&self, m: Media) -> bool {
        match m {
            Media::Video => self.enableVideo,
            Media::Audio => self.enableAudio,
            Media::Image => self.enableImage,
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct Outcome {
    pub path: String,
    pub state: &'static str,
    pub message: String,
    pub old_md5: Option<String>,
    pub new_md5: Option<String>,
}
impl Outcome {
    pub fn new(path: impl Into<String>, state: &'static str, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            state,
            message: message.into(),
            old_md5: None,
            new_md5: None,
        }
    }
}
pub async fn run(
    ctx: &ExtensionContext,
    command: &str,
    d: &Draft,
    mut record: impl FnMut(Outcome),
) -> Result<()> {
    d.validate(command)?;
    if command == "zipExtract" {
        let paths: Vec<_> = d
            .paths
            .iter()
            .filter(|p| {
                Path::new(p)
                    .extension()
                    .is_some_and(|s| s.eq_ignore_ascii_case("zip"))
            })
            .cloned()
            .collect();
        if paths.is_empty() {
            record(Outcome::new(
                "ZIP selection",
                "skipped",
                "No zip file selected",
            ));
            return Ok(());
        }
        match extract(ctx, &paths).await {
            Ok(p) => record(Outcome::new(p, "committed", "ZIP extraction published")),
            Err(e) => record(Outcome::new(paths.join("; "), "failed", e)),
        };
        return Ok(());
    }
    let mut walk = Walk::default();
    for p in &d.paths {
        fs::checkpoint(ctx).await?;
        if let Err(e) = walk.selected(Path::new(p)) {
            walk.failures.push((p.into(), e));
        }
    }
    let display_budget: usize = walk
        .nodes
        .iter()
        .map(|n| {
            serde_json::to_string(&n.path.to_string_lossy())
                .map_or(usize::MAX / 8192, |v| v.len() * 2 + 4608)
        })
        .sum::<usize>()
        + walk
            .failures
            .iter()
            .map(|(p, _)| {
                serde_json::to_string(&p.to_string_lossy())
                    .map_or(usize::MAX / 8192, |v| v.len() * 2 + 4608)
            })
            .sum::<usize>();
    if display_budget > 4 * 1024 * 1024 {
        return Err(
            "Batch report would exceed 4 MiB; submit fewer/shorter paths (nothing attempted)"
                .into(),
        );
    }
    if command == "zipCompress" {
        if !walk.failures.is_empty() {
            for (p, e) in walk.failures {
                record(Outcome::new(p.to_string_lossy(), "failed", e));
            }
            return Ok(());
        }
        match compress(ctx, &d.paths, &walk.nodes).await {
            Ok(p) => record(Outcome::new(p, "committed", "ZIP archive published")),
            Err(e) => record(Outcome::new(d.paths.join("; "), "failed", e)),
        };
        return Ok(());
    }
    for (p, e) in walk.failures {
        record(Outcome::new(p.to_string_lossy(), "failed", e));
    }
    let mut seen = HashSet::new();
    for n in walk.nodes.iter().filter(|n| n.meta.is_file()) {
        fs::checkpoint(ctx).await?;
        // Aliased selected spellings are handled once per canonical inode identity.
        #[cfg(unix)]
        let key = {
            use cap_std::fs::MetadataExt;
            format!("{}:{}", n.meta.dev(), n.meta.ino())
        };
        #[cfg(windows)]
        let key = {
            use cap_primitives::fs::_WindowsByHandle;
            format!(
                "{:?}:{:?}",
                n.meta.volume_serial_number(),
                n.meta.file_index()
            )
        };
        if !seen.insert(key) {
            record(Outcome::new(
                n.path.to_string_lossy(),
                "skipped",
                "Duplicate selected identity",
            ));
            continue;
        }
        let o = match hash_file(ctx, n, command == "modifyHash", d).await {
            Ok(o) => o,
            Err(e) => Outcome::new(n.path.to_string_lossy(), "failed", e),
        };
        record(o);
    }
    Ok(())
}
async fn hash_file(ctx: &ExtensionContext, n: &Node, modify: bool, d: &Draft) -> Result<Outcome> {
    let mut input = n.open()?;
    let mut probe = vec![];
    std::io::Read::by_ref(&mut input)
        .take(16 * 1024 * 1024)
        .read_to_end(&mut probe)
        .map_err(fs::err)?;
    let media = media::classify(&probe, n.meta.len())?;
    drop(probe);
    let mut o = Outcome::new(
        n.path.to_string_lossy(),
        "skipped",
        "Not an enabled media type",
    );
    if !media.is_some_and(|m| d.enabled(m)) {
        return Ok(o);
    }
    let len = n.meta.len();
    let mut out_len = len;
    if !modify {
        if len < MARKER.len() as u64 {
            return unchanged_file(ctx, n, input, d.showMD5Log).await;
        }
        input
            .seek(SeekFrom::End(-(MARKER.len() as i64)))
            .map_err(fs::err)?;
        let mut tail = [0u8; 8];
        input.read_exact(&mut tail).map_err(fs::err)?;
        if tail != MARKER {
            return unchanged_file(ctx, n, input, d.showMD5Log).await;
        }
        out_len -= MARKER.len() as u64;
    }
    if n.meta.permissions().readonly() {
        return Err("Source is read-only".into());
    }
    input.rewind().map_err(fs::err)?;
    let (mut stage, mut output) = Stage::file(n.dir.clone())?;
    // Hash original and output in the same pass; never reread an acknowledged mutation.
    let mut old = md5::Context::new();
    let mut new = md5::Context::new();
    let mut remaining = len;
    let mut written = 0u64;
    let mut buf = vec![0u8; 256 * 1024];
    while remaining > 0 {
        fs::checkpoint(ctx).await?;
        let count = input.read(&mut buf).map_err(fs::err)?;
        if count == 0 {
            return Err("Source shrank during read".into());
        }
        if count as u64 > remaining {
            return Err("Source grew during read".into());
        }
        if d.showMD5Log {
            old.consume(&buf[..count]);
        }
        let emit = (out_len - written).min(count as u64) as usize;
        if emit > 0 {
            output.write_all(&buf[..emit]).map_err(fs::err)?;
            if d.showMD5Log {
                new.consume(&buf[..emit]);
            }
            written += emit as u64;
        }
        remaining -= count as u64;
    }
    if input.read(&mut [0u8; 1]).map_err(fs::err)? != 0 {
        return Err("Source grew during read".into());
    }
    if modify {
        output.write_all(MARKER).map_err(fs::err)?;
        if d.showMD5Log {
            new.consume(MARKER);
        }
    }
    output
        .set_permissions(n.meta.permissions())
        .map_err(fs::err)?;
    output.sync_all().map_err(fs::err)?;
    if !fs::same(&n.meta, &input.metadata().map_err(fs::err)?) {
        return Err("Source changed during read".into());
    }
    fs::checkpoint(ctx).await?;
    stage.replace(n)?;
    // No suspension between the commit and recording this return value.
    o.state = "committed";
    o.message = if modify {
        "Appended one #$1024$# marker"
    } else {
        "Removed one trailing #$1024$# marker"
    }
    .into();
    if d.showMD5Log {
        o.old_md5 = Some(format!("{:x}", old.compute()));
        o.new_md5 = Some(format!("{:x}", new.compute()));
    }
    Ok(o)
}
async fn unchanged_file(
    ctx: &ExtensionContext,
    n: &Node,
    mut input: cap_std::fs::File,
    show: bool,
) -> Result<Outcome> {
    let mut o = Outcome::new(
        n.path.to_string_lossy(),
        "unchanged",
        "No trailing marker; all bytes preserved",
    );
    if show {
        input.rewind().map_err(fs::err)?;
        let (size, hash) = fs::transfer(ctx, &mut input, &mut std::io::sink(), MAX_FILE).await?;
        if size != n.meta.len() || !fs::same(&n.meta, &input.metadata().map_err(fs::err)?) {
            return Err("Source changed during MD5 read".into());
        }
        n.check()?;
        o.old_md5 = Some(hash.clone());
        o.new_md5 = Some(hash);
    }
    Ok(o)
}
fn zip_name(paths: &[String]) -> Result<PathBuf> {
    let normalized: PathBuf = Path::new(paths.first().ok_or("Empty selection")?)
        .components()
        .collect();
    let p = normalized.as_path();
    if paths.len() == 1 {
        let mut s = p.as_os_str().to_owned();
        s.push(".zip");
        Ok(s.into())
    } else {
        let stem = p.file_stem().ok_or("Missing file name")?;
        let mut n = stem.to_os_string();
        n.push(format!(" ({} files).zip", paths.len()));
        Ok(p.with_file_name(n))
    }
}
pub fn member(name: &str) -> Result<(String, bool)> {
    if name.is_empty()
        || name.len() > 4096
        || name.starts_with('/')
        || name.contains(['\\', ':', '\0'])
    {
        return Err(format!("Unsafe ZIP member: {name:?}"));
    }
    let directory = name.ends_with('/');
    let clean = name.strip_suffix('/').unwrap_or(name);
    let parts: Vec<_> = clean.split('/').collect();
    if parts.len() > 64 {
        return Err("ZIP member exceeds 64 levels".into());
    }
    for part in &parts {
        if part.is_empty()
            || *part == "."
            || *part == ".."
            || part.ends_with(['.', ' '])
            || part.chars().any(|c| c < ' ')
        {
            return Err(format!("Unsafe ZIP component: {part:?}"));
        }
        let base = part.split('.').next().unwrap().to_ascii_uppercase();
        if ["CON", "PRN", "AUX", "NUL"].contains(&base.as_str())
            || (base.len() == 4
                && (base.starts_with("COM") || base.starts_with("LPT"))
                && base.as_bytes()[3].is_ascii_digit()
                && base.as_bytes()[3] != b'0')
        {
            return Err(format!("Windows device ZIP member: {part:?}"));
        }
    }
    Ok((clean.into(), directory))
}
fn register(names: &mut HashMap<String, (bool, bool)>, name: &str, directory: bool) -> Result<()> {
    let key = name.to_lowercase();
    if let Some((old_dir, explicit)) = names.get(&key) {
        if !directory || !*old_dir || *explicit {
            return Err(format!("Duplicate/conflicting ZIP member: {name}"));
        }
    }
    let parts: Vec<_> = key.split('/').collect();
    for i in 1..parts.len() {
        let ancestor = parts[..i].join("/");
        if names.get(&ancestor).is_some_and(|(dir, _)| !*dir) {
            return Err(format!("ZIP file used as directory: {ancestor}"));
        }
        names.entry(ancestor).or_insert((true, false));
    }
    if names.len() > MAX_ITEMS {
        return Err("ZIP exceeds 4096 explicit/implicit paths".into());
    }
    names.insert(key, (directory, true));
    Ok(())
}
async fn compress(ctx: &ExtensionContext, paths: &[String], nodes: &[Node]) -> Result<String> {
    let dest = zip_name(paths)?;
    let (parent, name) = fs::parent(&dest)?;
    let (mut stage, output) = Stage::file(parent)?;
    let mut zip = ZipWriter::new(output);
    let mut names = HashMap::new();
    for n in nodes {
        fs::checkpoint(ctx).await?;
        if n.path.to_str().is_none() {
            return Err("Non-UTF-8 ZIP source name".into());
        }
        let (clean, directory) = member(&n.archive)?;
        register(&mut names, &clean, directory)?;
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .last_modified_time(zip::DateTime::default())
            .unix_permissions(if directory { 0o755 } else { 0o644 })
            .large_file(true);
        if directory {
            zip.add_directory(format!("{clean}/"), options)
                .map_err(fs::err)?;
        } else {
            let mut input = n.open()?;
            zip.start_file(clean, options).map_err(fs::err)?;
            fs::transfer(ctx, &mut input, &mut zip, MAX_FILE).await?;
            n.check()?;
        }
    }
    let output = zip.finish().map_err(fs::err)?;
    if output.metadata().map_err(fs::err)?.len() > MAX_TOTAL {
        return Err("ZIP output exceeds 16 GiB".into());
    }
    output.sync_all().map_err(fs::err)?;
    drop(output);
    fs::checkpoint(ctx).await?;
    let published = stage.publish(&name)?;
    Ok(dest.with_file_name(published).to_string_lossy().into())
}
fn ensure_dir(root: &Dir, path: &Path) -> Result<Dir> {
    let mut d = root.try_clone().map_err(fs::err)?;
    for c in path.components() {
        if let std::path::Component::Normal(n) = c {
            match d.create_dir(n) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(e) => return Err(fs::err(e)),
            }
            d = d.open_dir_nofollow(n).map_err(fs::err)?;
        }
    }
    Ok(d)
}
// zip 2.x collapses exact duplicate names internally. Validate the raw bounded
// central directory BEFORE constructing it, so duplicates and forged counts
// cannot disappear from the safety checks or allocate an unbounded index.
fn central_preflight(source: &mut cap_std::fs::File) -> Result<usize> {
    let length = source.metadata().map_err(fs::err)?.len();
    let tail_len = length.min(1024 * 1024) as usize;
    source
        .seek(SeekFrom::End(-(tail_len as i64)))
        .map_err(fs::err)?;
    let mut tail = vec![0; tail_len];
    source.read_exact(&mut tail).map_err(fs::err)?;
    let pos = (0..tail.len().saturating_sub(21))
        .rev()
        .find(|p| {
            tail.get(*p..*p + 4) == Some(b"PK\x05\x06")
                && tail.get(*p + 20..*p + 22).is_some_and(|n| {
                    *p + 22 + u16::from_le_bytes(n.try_into().unwrap()) as usize == tail.len()
                })
        })
        .ok_or("ZIP end record missing, truncated comment, or trailing bytes")?;
    let e = tail.get(pos..pos + 22).ok_or("Truncated ZIP end record")?;
    let u16le = |s: &[u8]| u16::from_le_bytes(s.try_into().unwrap());
    let u32le = |s: &[u8]| u32::from_le_bytes(s.try_into().unwrap());
    let u64le = |s: &[u8]| u64::from_le_bytes(s.try_into().unwrap());
    if u16le(&e[4..6]) != 0 || u16le(&e[6..8]) != 0 {
        return Err("Multi-disk ZIP archives are unsupported".into());
    }
    let mut count = u16le(&e[10..12]) as u64;
    let mut size = u32le(&e[12..16]) as u64;
    let mut end = length - tail_len as u64 + pos as u64;
    if count == 65535 || size == u32::MAX as u64 || u32le(&e[16..20]) == u32::MAX {
        let locator = pos.checked_sub(20).ok_or("ZIP64 locator missing")?;
        if tail.get(locator..locator + 4) != Some(b"PK\x06\x07") {
            return Err("ZIP64 locator missing".into());
        }
        let record = tail[..locator]
            .windows(4)
            .rposition(|s| s == b"PK\x06\x06")
            .ok_or("ZIP64 record exceeds 1 MiB probe")?;
        let h = tail
            .get(record..record + 56)
            .ok_or("Truncated ZIP64 record")?;
        if u32le(&h[16..20]) != 0 || u32le(&h[20..24]) != 0 {
            return Err("Multi-disk ZIP64 unsupported".into());
        }
        count = u64le(&h[32..40]);
        size = u64le(&h[40..48]);
        end = length - tail_len as u64 + record as u64;
    }
    if count > MAX_ITEMS as u64 || size > 16 * 1024 * 1024 {
        return Err("ZIP central directory exceeds 4096 entries or 16 MiB".into());
    }
    let start = end.checked_sub(size).ok_or("Invalid ZIP directory range")?;
    source.seek(SeekFrom::Start(start)).map_err(fs::err)?;
    let mut central = vec![0; size as usize];
    source.read_exact(&mut central).map_err(fs::err)?;
    let mut p = 0usize;
    let mut seen = HashSet::new();
    let mut actual = 0usize;
    while p < central.len() {
        let h = central
            .get(p..p + 46)
            .ok_or("Truncated central directory")?;
        if &h[..4] != b"PK\x01\x02" {
            return Err("Unexpected central directory record".into());
        }
        let n = u16le(&h[28..30]) as usize;
        let extra = u16le(&h[30..32]) as usize;
        let comment = u16le(&h[32..34]) as usize;
        if n > 4096 {
            return Err("ZIP filename exceeds 4096 bytes".into());
        }
        let name = central
            .get(p + 46..p + 46 + n)
            .ok_or("ZIP filename truncated")?;
        if !seen.insert(name.to_vec()) {
            return Err("Duplicate raw ZIP member".into());
        }
        actual += 1;
        if actual > MAX_ITEMS {
            return Err("ZIP entry limit".into());
        }
        p = p
            .checked_add(46 + n + extra + comment)
            .ok_or("ZIP directory overflow")?;
    }
    if actual as u64 != count || p != central.len() {
        return Err("ZIP central directory count/range mismatch".into());
    }
    source.rewind().map_err(fs::err)?;
    Ok(actual)
}
struct Entry {
    name: String,
    directory: bool,
    size: u64,
    compressed: u64,
    start: u64,
    crc: u32,
    method: CompressionMethod,
}
fn plan(archive: &mut ZipArchive<cap_std::fs::File>) -> Result<Vec<Entry>> {
    if archive.len() > MAX_ITEMS {
        return Err("ZIP exceeds 4096 entries".into());
    }
    let mut out = vec![];
    for i in 0..archive.len() {
        let e = archive.by_index(i).map_err(fs::err)?;
        if e.encrypted() {
            return Err("Encrypted ZIP entries are unsupported".into());
        }
        if !matches!(
            e.compression(),
            CompressionMethod::Stored | CompressionMethod::Deflated
        ) {
            return Err("ZIP compression method is unsupported (Stored/Deflate only)".into());
        }
        let kind = e.unix_mode().unwrap_or(0) & 0o170000;
        if !matches!(kind, 0 | 0o100000 | 0o040000) {
            return Err("ZIP links/special entries are rejected".into());
        }
        let (name, directory) = member(e.name())?;
        if kind == 0o040000 && !directory {
            return Err("ZIP directory mode/name mismatch".into());
        }
        if e.size() > MAX_FILE
            || e.size()
                > e.compressed_size()
                    .saturating_mul(1000)
                    .saturating_add(1024 * 1024)
        {
            return Err("ZIP entry exceeds 8 GiB or 1000:1 + 1 MiB expansion bound".into());
        }
        out.push(Entry {
            name,
            directory,
            size: e.size(),
            compressed: e.compressed_size(),
            start: e.data_start(),
            crc: e.crc32(),
            method: e.compression(),
        });
    }
    Ok(out)
}
struct CrcRead<R> {
    reader: R,
    crc: crc32fast::Hasher,
}
impl<R: Read> Read for CrcRead<R> {
    fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
        let n = self.reader.read(b)?;
        self.crc.update(&b[..n]);
        Ok(n)
    }
}
async fn extract(ctx: &ExtensionContext, paths: &[String]) -> Result<String> {
    let zipped = zip_name(paths)?;
    let base = zipped
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid ZIP output name")?
        .split('.')
        .next()
        .ok_or("Invalid ZIP output name")?;
    if base.is_empty() {
        return Err(
            "V1's first-dot extraction name is empty; rename the source archive explicitly".into(),
        );
    }
    let dest = zipped.with_file_name(base);
    let (parent, name) = fs::parent(&dest)?;
    let (mut stage, root) = Stage::directory(parent)?;
    let mut names = HashMap::new();
    let mut total = 0u64;
    let mut count = 0usize;
    for path in paths {
        let (d, n) = fs::parent(Path::new(path))?;
        let meta = d.symlink_metadata(&n).map_err(fs::err)?;
        fs::ordinary(&meta)?;
        if !meta.is_file() || meta.len() > MAX_FILE {
            return Err("ZIP source must be a regular file of at most 8 GiB".into());
        }
        let node = Node {
            dir: d,
            name: n,
            path: path.into(),
            archive: String::new(),
            meta,
        };
        let mut input = node.open()?;
        let raw_count = central_preflight(&mut input)?;
        let mut archive = ZipArchive::new(input).map_err(fs::err)?;
        if archive.len() != raw_count {
            return Err("ZIP decoded member names conflict".into());
        }
        let entries = plan(&mut archive)?;
        let mut source = archive.into_inner();
        count = count
            .checked_add(entries.len())
            .ok_or("ZIP count overflow")?;
        if count > MAX_ITEMS {
            return Err("ZIP exceeds 4096 entries across the batch".into());
        }
        for entry in entries {
            fs::checkpoint(ctx).await?;
            register(&mut names, &entry.name, entry.directory)?;
            total = total.checked_add(entry.size).ok_or("ZIP size overflow")?;
            if total > MAX_TOTAL {
                return Err("ZIP expands beyond 16 GiB".into());
            }
            if entry.directory {
                ensure_dir(&root, Path::new(&entry.name))?;
            } else {
                let p = Path::new(&entry.name);
                let dir = ensure_dir(&root, p.parent().unwrap_or(Path::new("")))?;
                let mut f = dir
                    .open_with(p.file_name().unwrap(), &fs::options(true, true))
                    .map_err(fs::err)?;
                source.seek(SeekFrom::Start(entry.start)).map_err(fs::err)?;
                let limited = std::io::Read::by_ref(&mut source).take(entry.compressed);
                let reader: Box<dyn Read + Send + '_> = match entry.method {
                    CompressionMethod::Stored => Box::new(limited),
                    _ => Box::new(flate2::read::DeflateDecoder::new(limited)),
                };
                let mut reader = CrcRead {
                    reader,
                    crc: crc32fast::Hasher::new(),
                };
                let (actual, _) = fs::transfer(ctx, &mut reader, &mut f, entry.size).await?;
                if actual != entry.size || reader.crc.finalize() != entry.crc {
                    return Err("ZIP size/CRC does not match central directory".into());
                }
                f.sync_all().map_err(fs::err)?;
            }
        }
        node.check()?;
    }
    drop(root);
    fs::checkpoint(ctx).await?;
    let published = stage.publish(&name)?;
    Ok(dest.with_file_name(published).to_string_lossy().into())
}
