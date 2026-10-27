//! All product I/O is handle relative. Ambient authority is used only to acquire
//! a filesystem root; every user-controlled component is opened no-follow.
use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};
#[cfg(windows)]
use cap_primitives::fs::_WindowsByHandle;
use cap_std::fs::{Dir, File, Metadata, MetadataExt, OpenOptions};
use std::{
    ffi::OsString,
    io::{self, Read, Write},
    path::{Component, Path, PathBuf},
    sync::Arc,
};
use uuid::Uuid;

pub type Result<T> = std::result::Result<T, String>;
pub fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
pub const MAX_FILE: u64 = 8 * 1024 * 1024 * 1024;
pub const MAX_TOTAL: u64 = 16 * 1024 * 1024 * 1024;
pub const MAX_ITEMS: usize = 4096;
pub fn ordinary(m: &Metadata) -> Result<()> {
    if m.file_type().is_symlink() || (!m.is_file() && !m.is_dir()) {
        return Err("Only ordinary files/directories; links and special files are rejected".into());
    }
    #[cfg(windows)]
    if MetadataExt::file_attributes(m) & 0x400 != 0 {
        return Err("Reparse points are rejected".into());
    }
    Ok(())
}
pub fn same(a: &Metadata, b: &Metadata) -> bool {
    #[cfg(unix)]
    {
        a.dev() == b.dev()
            && a.ino() == b.ino()
            && a.len() == b.len()
            && a.modified().ok() == b.modified().ok()
    }
    #[cfg(windows)]
    {
        a.volume_serial_number().is_some()
            && a.file_index().is_some()
            && a.volume_serial_number() == b.volume_serial_number()
            && a.file_index() == b.file_index()
            && a.len() == b.len()
            && a.modified().ok() == b.modified().ok()
    }
}
pub fn options(write: bool, create: bool) -> OpenOptions {
    let mut o = OpenOptions::new();
    #[cfg(unix)]
    {
        use cap_std::fs::OpenOptionsExt;
        o.custom_flags(rustix::fs::OFlags::NONBLOCK.bits() as i32);
        if create {
            o.mode(0o600);
        }
    }

    o.read(true)
        .write(write)
        .create_new(create)
        .follow(FollowSymlinks::No);
    o
}
pub fn absolute(p: &Path) -> Result<()> {
    if !p.is_absolute() || p.as_os_str().len() > 32768 {
        return Err("Supply an absolute path of at most 32768 bytes".into());
    }
    if p.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err("Parent traversal is rejected".into());
    }
    #[cfg(windows)]
    {
        use std::path::Prefix;
        if p.components().any(|c|matches!(c,Component::Prefix(x) if !matches!(x.kind(),Prefix::Disk(_)|Prefix::UNC(_, _)))){return Err("Device/verbatim paths are rejected".into());}
    }
    Ok(())
}
pub fn parent(p: &Path) -> Result<(Arc<Dir>, OsString)> {
    absolute(p)?;
    let mut root = PathBuf::new();
    let mut names = vec![];
    for c in p.components() {
        match c {
            Component::Prefix(_) | Component::RootDir => root.push(c.as_os_str()),
            Component::Normal(n) => names.push(n.to_owned()),
            Component::CurDir => {}
            Component::ParentDir => unreachable!(),
        }
    }
    let name = names.pop().ok_or("A filesystem root cannot be selected")?;
    let mut d = Dir::open_ambient_dir(root, cap_std::ambient_authority()).map_err(err)?;
    for n in names {
        ordinary(&d.symlink_metadata(&n).map_err(err)?)?;
        d = d.open_dir_nofollow(n).map_err(err)?;
        ordinary(&d.dir_metadata().map_err(err)?)?;
    }
    Ok((Arc::new(d), name))
}
pub struct Node {
    pub dir: Arc<Dir>,
    pub name: OsString,
    pub path: PathBuf,
    pub archive: String,
    pub meta: Metadata,
}
impl Node {
    pub fn open(&self) -> Result<File> {
        let f = self
            .dir
            .open_with(&self.name, &options(false, false))
            .map_err(err)?;
        let m = f.metadata().map_err(err)?;
        ordinary(&m)?;
        if !m.is_file() || !same(&self.meta, &m) {
            return Err("Source identity changed; submit a new batch after checking it".into());
        }
        Ok(f)
    }
    pub fn check(&self) -> Result<()> {
        let m = self.dir.symlink_metadata(&self.name).map_err(err)?;
        ordinary(&m)?;
        if same(&self.meta, &m) {
            Ok(())
        } else {
            Err("Source changed before commit".into())
        }
    }
}
#[derive(Default)]
pub struct Walk {
    pub nodes: Vec<Node>,
    pub failures: Vec<(PathBuf, String)>,
    pub total: u64,
    pub visited: usize,
}
impl Walk {
    pub fn selected(&mut self, p: &Path) -> Result<()> {
        let (d, n) = parent(p)?;
        let a = n
            .to_str()
            .ok_or("ZIP member filename is not UTF-8")?
            .to_owned();
        self.walk(d, n, p.to_owned(), a, 0)
    }
    fn walk(
        &mut self,
        d: Arc<Dir>,
        n: OsString,
        p: PathBuf,
        a: String,
        depth: usize,
    ) -> Result<()> {
        self.visited += 1;
        if self.visited > MAX_ITEMS || depth > 64 {
            return Err("Batch exceeds 4096 items or 64 directory levels".into());
        }
        let m = d.symlink_metadata(&n).map_err(err)?;
        ordinary(&m)?;
        if m.is_file() {
            if m.len() > MAX_FILE {
                return Err("File exceeds 8 GiB".into());
            }
            self.total = self
                .total
                .checked_add(m.len())
                .ok_or("Batch size overflow")?;
            if self.total > MAX_TOTAL {
                return Err("Batch exceeds 16 GiB".into());
            }
            self.nodes.push(Node {
                dir: d,
                name: n,
                path: p,
                archive: a,
                meta: m,
            });
            return Ok(());
        }
        let child = Arc::new(d.open_dir_nofollow(&n).map_err(err)?);
        ordinary(&child.dir_metadata().map_err(err)?)?;
        self.nodes.push(Node {
            dir: d,
            name: n,
            path: p.clone(),
            archive: format!("{a}/"),
            meta: m,
        });
        let mut entries = child
            .entries()
            .map_err(err)?
            .take(MAX_ITEMS + 1)
            .collect::<io::Result<Vec<_>>>()
            .map_err(err)?;
        entries.sort_by_key(|e| e.file_name());
        if entries.len() > MAX_ITEMS {
            return Err("Directory exceeds 4096 entries".into());
        }
        for e in entries {
            let name = e.file_name();
            let path = p.join(&name);
            if self.visited >= MAX_ITEMS {
                return Err("Batch exceeds 4096 items".into());
            }
            let arc = format!("{a}/{}", name.to_string_lossy());
            if let Err(e) = self.walk(child.clone(), name, path.clone(), arc, depth + 1) {
                self.failures.push((path, e));
            }
        }
        Ok(())
    }
}

pub struct Stage {
    pub dir: Arc<Dir>,
    pub name: OsString,
    pub directory: bool,
    pub live: bool,
}
impl Stage {
    pub fn file(dir: Arc<Dir>) -> Result<(Self, File)> {
        let n = OsString::from(format!(".modify-hash-{}.tmp", Uuid::new_v4()));
        let f = dir.open_with(&n, &options(true, true)).map_err(err)?;
        Ok((
            Self {
                dir,
                name: n,
                directory: false,
                live: true,
            },
            f,
        ))
    }
    pub fn directory(dir: Arc<Dir>) -> Result<(Self, Dir)> {
        let n = OsString::from(format!(".modify-hash-{}.tmp", Uuid::new_v4()));
        let builder = cap_std::fs::DirBuilder::new();
        #[cfg(unix)]
        let builder = {
            use cap_std::fs::DirBuilderExt;
            let mut b = builder;
            b.mode(0o700);
            b
        };
        dir.create_dir_with(&n, &builder).map_err(err)?;
        let d = dir.open_dir_nofollow(&n).map_err(err)?;
        Ok((
            Self {
                dir,
                name: n,
                directory: true,
                live: true,
            },
            d,
        ))
    }
    pub fn replace(&mut self, node: &Node) -> Result<()> {
        node.check()?;
        self.dir
            .rename(&self.name, &node.dir, &node.name)
            .map_err(err)?;
        self.live = false;
        Ok(())
    }
    pub fn publish(&mut self, base: &OsString) -> Result<OsString> {
        let path = Path::new(base);
        let stem = path.file_stem().unwrap_or(base);
        let ext = path.extension();
        for i in 1..=10000 {
            let name = if i == 1 {
                base.clone()
            } else {
                let mut n = stem.to_os_string();
                n.push(format!(" {i}"));
                if let Some(e) = ext {
                    n.push(".");
                    n.push(e);
                }
                n
            };
            match self.dir.symlink_metadata(&name) {
                Ok(_) => continue,
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => return Err(err(e)),
            }
            match rename_new(&self.dir, &self.name, &name) {
                Ok(()) => {
                    self.live = false;
                    return Ok(name);
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(err(e)),
            }
        }
        Err("No free output name after 10000 collisions".into())
    }
}
impl Drop for Stage {
    fn drop(&mut self) {
        if self.live {
            if self.directory {
                let _ = self.dir.remove_dir_all(&self.name);
            } else {
                let _ = self.dir.remove_file(&self.name);
            }
        }
    }
}
#[cfg(unix)]
fn rename_new(d: &Dir, src: &OsString, dst: &OsString) -> io::Result<()> {
    rustix::fs::renameat_with(d, src, d, dst, rustix::fs::RenameFlags::NOREPLACE)
        .map_err(Into::into)
}
#[cfg(windows)]
fn rename_new(d: &Dir, src: &OsString, dst: &OsString) -> io::Result<()> {
    use cap_std::fs::OpenOptionsExt;
    use std::os::windows::{ffi::OsStrExt, io::AsRawHandle};
    use windows_sys::Win32::Storage::FileSystem::{
        FileRenameInfo, SetFileInformationByHandle, DELETE, FILE_FLAG_BACKUP_SEMANTICS,
        FILE_FLAG_OPEN_REPARSE_POINT, FILE_READ_ATTRIBUTES, FILE_RENAME_INFO,
    };
    let mut o = options(false, false);
    o.access_mode(DELETE | FILE_READ_ATTRIBUTES)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT);
    let f = d.open_with(src, &o)?;
    ordinary(&f.metadata()?).map_err(io::Error::other)?;
    let wide: Vec<u16> = dst.encode_wide().collect();
    let bytes = std::mem::size_of::<FILE_RENAME_INFO>() + wide.len() * 2;
    let mut buf = vec![0usize; bytes.div_ceil(std::mem::size_of::<usize>())];
    // Aligned variable-size Windows FILE_RENAME_INFO, relative to pinned parent.
    unsafe {
        let info = buf.as_mut_ptr().cast::<FILE_RENAME_INFO>();
        (*info).Anonymous.ReplaceIfExists = 0;
        (*info).RootDirectory = d.as_raw_handle();
        (*info).FileNameLength = (wide.len() * 2) as u32;
        std::ptr::copy_nonoverlapping(wide.as_ptr(), (*info).FileName.as_mut_ptr(), wide.len());
        if SetFileInformationByHandle(f.as_raw_handle(), FileRenameInfo, info.cast(), bytes as u32)
            == 0
        {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
}
pub async fn checkpoint(ctx: &tezbar_extension_sdk::ExtensionContext) -> Result<()> {
    tokio::select! {biased;_=ctx.cancelled()=>Err("Cancelled; committed items remain recorded, no automatic replay".into()),_=tokio::task::yield_now()=>Ok(())}
}
pub async fn transfer<R: Read, W: Write>(
    ctx: &tezbar_extension_sdk::ExtensionContext,
    r: &mut R,
    w: &mut W,
    limit: u64,
) -> Result<(u64, String)> {
    let mut n = 0u64;
    let mut md5 = md5::Context::new();
    let mut buf = vec![0u8; 256 * 1024];
    loop {
        checkpoint(ctx).await?;
        let k = r.read(&mut buf).map_err(err)?;
        if k == 0 {
            break;
        }
        n += k as u64;
        if n > limit {
            return Err(format!("Stream exceeds {limit} bytes"));
        }
        w.write_all(&buf[..k]).map_err(err)?;
        md5.consume(&buf[..k]);
    }
    Ok((n, format!("{:x}", md5.compute())))
}
