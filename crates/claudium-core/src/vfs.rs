//! The prepared resources' small files as one file, `claudium-bundle.bin`, read once.
//!
//! Prepared resources are about 20,000 small JSON and PNG files under `assets/` and `data/`.
//! Reaching the title screen opens about 26,000 of them and entering a world 16,000 more.
//! That costs a few seconds on Linux and macOS but half a minute on Windows, where every
//! file open is slow and goes past the antivirus. Java Edition never pays it: it reads one jar.
//!
//! [`mount`] packs those files into one bundle beside them the first time, then reads the bundle
//! into memory on every later launch. This module mirrors the parts of `std::fs` the resource
//! loaders use, so a loader moves over by importing `claudium_core::vfs as fs`: reads of a
//! mounted root come from memory, and everything else (writes, files outside a mounted root or
//! not in the bundle) goes to `std::fs` unchanged. A missing, stale or broken bundle only means
//! the files are read one by one, as before.
//!
//! The bundle is keyed on the prepared root's `claudium-resources.json` (its size and
//! modification time), which every preparation writes, so preparing again makes a new bundle.

use std::{
    collections::{HashMap, HashSet},
    ffi::OsString,
    hash::{DefaultHasher, Hash, Hasher},
    io,
    path::{Component, Path, PathBuf},
    sync::{Arc, RwLock},
};

pub use std::fs::{
    File, Metadata, OpenOptions, Permissions, canonicalize, copy, create_dir, create_dir_all,
    hard_link, metadata, read_link, remove_dir, remove_dir_all, remove_file, rename,
    set_permissions, symlink_metadata, write,
};

/// The bundle's file name, beside the prepared root it is of (`<root>.claudium-bundle.bin`).
pub const FILE: &str = "claudium-bundle.bin";

/// Where `root`'s bundle lives: beside the root, not in it. Writing into the root would change
/// its modification time, which a resource pack's overlay checks (`packs/cache.rs` records the
/// prepared root's directories as its dependencies): every new bundle then rebuilt the overlay,
/// which on Windows copies every file again.
fn bundle_path(root: &Path) -> PathBuf {
    match (root.parent(), root.file_name()) {
        (Some(parent), Some(name)) => {
            let mut file = name.to_os_string();
            file.push(".");
            file.push(FILE);
            parent.join(file)
        }
        _ => root.join(FILE),
    }
}
const MAGIC: &[u8; 4] = b"CLBN";
/// Bump when the format or what goes in changes.
const VERSION: u32 = 3;
/// The trees packed, under the root.
const TREES: [&str; 3] = ["assets", "data", "worldgen"];
/// Files at least this big stay loose: they are few and read when needed.
const MAX_FILE: u64 = 1 << 20;
/// Directories left loose whole: the sounds (hundreds of megabytes, played when needed, and
/// copied in after preparing, with their `sounds.json`, which stays loose too).
const LOOSE_DIRS: [&str; 1] = ["sounds"];
/// The file whose size and time say which preparation the bundle is of.
const KEY_FILE: &str = "claudium-resources.json";

struct Bundle {
    root: PathBuf,
    data: Vec<u8>,
    /// Relative path (`/`-separated) to its bytes in `data`.
    files: HashMap<String, (usize, usize)>,
    /// Relative directory to its entries: name and whether it is a directory.
    dirs: HashMap<String, Vec<(String, bool)>>,
    /// Directories holding a file left loose: listing them needs the disk.
    partial: HashSet<String>,
}

static MOUNTS: RwLock<Vec<Arc<Bundle>>> = RwLock::new(Vec::new());

/// Serves `root`'s small files from its bundle, making the bundle first when it is missing or
/// stale. Returns how many files the bundle holds, or why it isn't used (the files are then
/// read one by one).
pub fn mount(root: &Path) -> Result<usize, String> {
    if std::env::var_os("CLAUDIUM_NO_BUNDLE").is_some() {
        return Err("CLAUDIUM_NO_BUNDLE is set".into());
    }
    if MOUNTS.read().unwrap().iter().any(|b| b.root == root) {
        return Ok(0);
    }
    let bundle = parse(root, fresh_bytes(root)?).ok_or("the bundle is damaged")?;
    let count = bundle.files.len();
    MOUNTS.write().unwrap().push(Arc::new(bundle));
    Ok(count)
}

/// Makes `root`'s bundle if it is missing or stale, without serving it: the end of preparing,
/// so even the first launch after it reads one file. Returns how many files it holds.
pub fn write_bundle(root: &Path) -> Result<usize, String> {
    let bytes = fresh_bytes(root)?;
    Ok(parse(root, bytes)
        .ok_or("the bundle is damaged")?
        .files
        .len())
}

/// The bundle's bytes: the one on disk when it is of this preparation and no directory changed
/// since, else a new one, written beside the files (a read-only folder just means no bundle on
/// disk next time).
fn fresh_bytes(root: &Path) -> Result<Vec<u8>, String> {
    let key = key(root).ok_or_else(|| format!("{} has no {KEY_FILE}", root.display()))?;
    let path = bundle_path(root);
    // The first bundles were written inside the root: take one away.
    let old = root.join(FILE);
    if old.is_file() {
        let _ = std::fs::remove_file(old);
    }
    let fresh = |b: &Vec<u8>| {
        header_key(b) == Some(key)
            && parse(root, b.clone()).is_some_and(|bundle| tree_key(&bundle) == header_tree_key(b))
    };
    if let Some(bytes) = std::fs::read(&path).ok().filter(fresh) {
        return Ok(bytes);
    }
    let bytes = build(root, key)?;
    let mut tmp = path.clone().into_os_string();
    tmp.push(format!(".{}.tmp", std::process::id()));
    let tmp = PathBuf::from(tmp);
    if std::fs::write(&tmp, &bytes).is_ok() && std::fs::rename(&tmp, &path).is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    Ok(bytes)
}

/// Forgets every bundle (tests, and a game that prepares its resources again while running).
pub fn unmount_all() {
    MOUNTS.write().unwrap().clear();
}

/// `std::fs::read`, from a bundle when the file is in one.
pub fn read<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>> {
    let path = path.as_ref();
    match lookup(path) {
        Found::Bytes(bytes) => Ok(bytes),
        Found::Missing => Err(not_found()),
        Found::Unknown => std::fs::read(path),
    }
}

/// `std::fs::read_to_string`, from a bundle when the file is in one.
pub fn read_to_string<P: AsRef<Path>>(path: P) -> io::Result<String> {
    let path = path.as_ref();
    match lookup(path) {
        Found::Bytes(bytes) => {
            String::from_utf8(bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
        }
        Found::Missing => Err(not_found()),
        Found::Unknown => std::fs::read_to_string(path),
    }
}

/// Whether `path` is a file, asking the bundle first.
pub fn is_file<P: AsRef<Path>>(path: P) -> bool {
    let path = path.as_ref();
    in_bundle(path).unwrap_or_else(|| path.is_file())
}

/// `std::fs::read_dir`, listed from a bundle when it holds the whole directory.
pub fn read_dir<P: AsRef<Path>>(path: P) -> io::Result<ReadDir> {
    let path = path.as_ref();
    if let Some(entries) = listing(path) {
        let entries = entries
            .into_iter()
            .map(|(name, dir)| DirEntry {
                path: path.join(&name),
                name: OsString::from(name),
                dir,
                std: None,
            })
            .collect::<Vec<_>>();
        return Ok(ReadDir {
            inner: Inner::Bundle(entries.into_iter()),
        });
    }
    std::fs::read_dir(path).map(|r| ReadDir {
        inner: Inner::Std(r),
    })
}

/// `std::fs::ReadDir`, from a bundle or the disk.
pub struct ReadDir {
    inner: Inner,
}
enum Inner {
    Std(std::fs::ReadDir),
    Bundle(std::vec::IntoIter<DirEntry>),
}
impl Iterator for ReadDir {
    type Item = io::Result<DirEntry>;
    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            Inner::Bundle(entries) => entries.next().map(Ok),
            Inner::Std(r) => r.next().map(|e| {
                e.and_then(|e| {
                    let dir = e.file_type()?.is_dir();
                    Ok(DirEntry {
                        path: e.path(),
                        name: e.file_name(),
                        dir,
                        std: Some(e),
                    })
                })
            }),
        }
    }
}

/// `std::fs::DirEntry`, from a bundle or the disk.
pub struct DirEntry {
    path: PathBuf,
    name: OsString,
    dir: bool,
    std: Option<std::fs::DirEntry>,
}
impl DirEntry {
    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }
    pub fn file_name(&self) -> OsString {
        self.name.clone()
    }
    pub fn file_type(&self) -> io::Result<FileType> {
        Ok(FileType { dir: self.dir })
    }
    /// The disk's metadata (a bundle entry's is read from the disk).
    pub fn metadata(&self) -> io::Result<Metadata> {
        match &self.std {
            Some(e) => e.metadata(),
            None => std::fs::metadata(&self.path),
        }
    }
}

/// `std::fs::FileType`: a directory or a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileType {
    dir: bool,
}
impl FileType {
    pub fn is_dir(&self) -> bool {
        self.dir
    }
    pub fn is_file(&self) -> bool {
        !self.dir
    }
    pub fn is_symlink(&self) -> bool {
        false
    }
}

/// `path` relative to a mounted root, `/`-separated, and that bundle.
fn relative(path: &Path) -> Option<(Arc<Bundle>, String)> {
    let mounts = MOUNTS.read().ok()?;
    for bundle in mounts.iter() {
        if let Ok(rest) = path.strip_prefix(&bundle.root) {
            let mut parts = Vec::new();
            for c in rest.components() {
                match c {
                    Component::Normal(s) => parts.push(s.to_str()?),
                    Component::CurDir => {}
                    _ => return None,
                }
            }
            return Some((bundle.clone(), parts.join("/")));
        }
    }
    None
}

/// What a bundle knows of a file.
enum Found {
    Bytes(Vec<u8>),
    /// In a directory the bundle holds whole, and not there: no file on the disk either.
    Missing,
    /// Not the bundle's to say: ask the disk.
    Unknown,
}

fn not_found() -> io::Error {
    io::Error::new(io::ErrorKind::NotFound, "not in the prepared resources")
}

fn lookup(path: &Path) -> Found {
    let Some((bundle, rel)) = relative(path) else {
        return Found::Unknown;
    };
    if let Some(&(at, len)) = bundle.files.get(&rel) {
        return Found::Bytes(bundle.data[at..at + len].to_vec());
    }
    // A missing file (an optional `.mcmeta`, an override): answered without the disk when its
    // directory is one the bundle holds whole. Looking for absent files is common, and on
    // Windows each look is as slow as an open.
    let parent = rel.rsplit_once('/').map_or("", |(p, _)| p);
    let whole = bundle.dirs.contains_key(parent) && !bundle.partial.contains(parent);
    if whole && !bundle.dirs.contains_key(&rel) {
        Found::Missing
    } else {
        Found::Unknown
    }
}

fn in_bundle(path: &Path) -> Option<bool> {
    let (bundle, rel) = relative(path)?;
    bundle.files.contains_key(&rel).then_some(true)
}

fn listing(path: &Path) -> Option<Vec<(String, bool)>> {
    let (bundle, rel) = relative(path)?;
    if bundle.partial.contains(&rel) {
        return None;
    }
    bundle.dirs.get(&rel).cloned()
}

/// Which preparation the root holds: its key file's size and modification time.
fn key(root: &Path) -> Option<u64> {
    let meta = std::fs::metadata(root.join(KEY_FILE)).ok()?;
    let mut h = DefaultHasher::new();
    VERSION.hash(&mut h);
    meta.len().hash(&mut h);
    meta.modified().ok()?.hash(&mut h);
    Some(h.finish())
}

fn header_key(bytes: &[u8]) -> Option<u64> {
    if bytes.get(..4)? != MAGIC || u32::from_le_bytes(bytes.get(4..8)?.try_into().ok()?) != VERSION
    {
        return None;
    }
    Some(u64::from_le_bytes(bytes.get(8..16)?.try_into().ok()?))
}

/// Packs `root`'s trees: the header, then each file's path and length, then the bytes.
fn build(root: &Path, key: u64) -> Result<Vec<u8>, String> {
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    let mut loose: Vec<String> = Vec::new();
    fn walk(
        dir: &Path,
        rel: &str,
        files: &mut Vec<(String, Vec<u8>)>,
        loose: &mut Vec<String>,
    ) -> io::Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                loose.push(rel.to_owned());
                continue;
            };
            let rel_path = format!("{rel}/{name}");
            let kind = entry.file_type()?;
            if kind.is_dir() && LOOSE_DIRS.contains(&name.as_str()) {
                loose.push(rel_path);
            } else if kind.is_dir() {
                walk(&entry.path(), &rel_path, files, loose)?;
            } else if kind.is_file() && name != "sounds.json" && entry.metadata()?.len() < MAX_FILE
            {
                files.push((rel_path, std::fs::read(entry.path())?));
            } else {
                loose.push(rel_path);
            }
        }
        Ok(())
    }
    for tree in TREES {
        let dir = root.join(tree);
        if dir.is_dir() {
            walk(&dir, tree, &mut files, &mut loose).map_err(|e| e.to_string())?;
        }
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&key.to_le_bytes());
    // The directories' times, filled in once the index says which directories there are.
    out.extend_from_slice(&0u64.to_le_bytes());
    out.extend_from_slice(&(files.len() as u32).to_le_bytes());
    out.extend_from_slice(&(loose.len() as u32).to_le_bytes());
    for (path, bytes) in &files {
        out.extend_from_slice(&(path.len() as u16).to_le_bytes());
        out.extend_from_slice(path.as_bytes());
        out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    }
    for path in &loose {
        out.extend_from_slice(&(path.len() as u16).to_le_bytes());
        out.extend_from_slice(path.as_bytes());
    }
    for (_, bytes) in &files {
        out.extend_from_slice(bytes);
    }
    let bundle = parse(root, out.clone()).ok_or("the new bundle doesn't read back")?;
    let tree = tree_key(&bundle);
    out[16..24].copy_from_slice(&tree.to_le_bytes());
    Ok(out)
}

fn header_tree_key(bytes: &[u8]) -> u64 {
    bytes
        .get(16..24)
        .and_then(|b| b.try_into().ok())
        .map_or(0, u64::from_le_bytes)
}

/// The modification times of every directory the bundle lists (and the loose ones), which
/// change when a file is added to or taken out of them: copying the sounds or the fonts in
/// after preparing makes the bundle stale.
fn tree_key(bundle: &Bundle) -> u64 {
    let mut dirs: Vec<&String> = bundle.dirs.keys().collect();
    dirs.sort();
    let mut h = DefaultHasher::new();
    for dir in dirs {
        dir.hash(&mut h);
        std::fs::metadata(bundle.root.join(dir))
            .and_then(|m| m.modified())
            .ok()
            .hash(&mut h);
    }
    h.finish()
}

fn parse(root: &Path, data: Vec<u8>) -> Option<Bundle> {
    let u16_at = |b: &[u8], at: usize| {
        Some(u16::from_le_bytes(b.get(at..at + 2)?.try_into().ok()?) as usize)
    };
    let u32_at = |b: &[u8], at: usize| {
        Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?) as usize)
    };
    header_key(&data)?;
    let count = u32_at(&data, 24)?;
    let loose_count = u32_at(&data, 28)?;
    let mut at = 32;
    let mut index = Vec::with_capacity(count);
    for _ in 0..count {
        let len = u16_at(&data, at)?;
        let path = std::str::from_utf8(data.get(at + 2..at + 2 + len)?)
            .ok()?
            .to_owned();
        at += 2 + len;
        let size = u32_at(&data, at)?;
        at += 4;
        index.push((path, size));
    }
    let mut partial = HashSet::new();
    for _ in 0..loose_count {
        let len = u16_at(&data, at)?;
        let path = std::str::from_utf8(data.get(at + 2..at + 2 + len)?).ok()?;
        at += 2 + len;
        // Every directory above a loose file can't be listed from the bundle.
        let mut dir = path;
        while let Some((parent, _)) = dir.rsplit_once('/') {
            partial.insert(parent.to_owned());
            dir = parent;
        }
    }
    let mut files = HashMap::with_capacity(count);
    let mut dirs: HashMap<String, Vec<(String, bool)>> = HashMap::new();
    let mut seen_dirs = HashSet::new();
    for (path, size) in index {
        if at + size > data.len() {
            return None;
        }
        // The file under its directory, and each directory under its parent once.
        let mut child = path.as_str();
        let mut is_dir = false;
        while let Some((parent, name)) = child.rsplit_once('/') {
            if is_dir && !seen_dirs.insert(child.to_owned()) {
                break;
            }
            dirs.entry(parent.to_owned())
                .or_default()
                .push((name.to_owned(), is_dir));
            child = parent;
            is_dir = true;
        }
        files.insert(path, (at, size));
        at += size;
    }
    for entries in dirs.values_mut() {
        entries.sort();
    }
    Some(Bundle {
        root: root.to_path_buf(),
        data,
        files,
        dirs,
        partial,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh folder for one test, removed when dropped.
    struct Scratch(PathBuf);
    impl Scratch {
        fn new(name: &str) -> Self {
            // In a parent of its own: the bundle goes beside the root.
            let dir = std::env::temp_dir()
                .join(format!("claudium-vfs-{name}-{}", std::process::id()))
                .join("root");
            let _ = std::fs::remove_dir_all(dir.parent().unwrap());
            std::fs::create_dir_all(&dir).unwrap();
            Scratch(dir)
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(self.0.parent().unwrap());
        }
    }
    fn put(root: &Path, rel: &str, bytes: &[u8]) {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, bytes).unwrap();
    }

    #[test]
    fn a_bundle_serves_what_the_disk_holds() {
        let s = Scratch::new("serve");
        let root = &s.0;
        put(root, KEY_FILE, b"{}");
        put(
            root,
            "assets/minecraft/models/block/stone.json",
            b"{\"parent\":\"cube\"}",
        );
        put(root, "assets/minecraft/models/item/stick.json", b"{}");
        put(root, "data/minecraft/recipe/stick.json", b"{\"r\":1}");
        put(
            root,
            "assets/minecraft/sounds/big.ogg",
            &vec![0u8; MAX_FILE as usize],
        );
        let bytes = build(root, 7).unwrap();
        let b = parse(root, bytes).unwrap();
        assert_eq!(b.files.len(), 3, "the sounds stay loose");
        assert_eq!(
            b.dirs["assets/minecraft/models"],
            vec![("block".to_string(), true), ("item".to_string(), true)]
        );
        assert!(b.partial.contains("assets/minecraft") && b.partial.contains("assets"));
        assert!(
            !b.dirs.contains_key("assets/minecraft/sounds"),
            "the sounds are left to the disk"
        );
        assert!(!b.partial.contains("assets/minecraft/models"));
        // Mounted: reads come from memory, and match the disk.
        assert_eq!(mount(root).unwrap(), 3);
        assert!(bundle_path(root).is_file(), "written beside the root");
        assert!(!root.join(FILE).exists(), "not inside it");
        let stone = root.join("assets/minecraft/models/block/stone.json");
        std::fs::remove_file(&stone).unwrap();
        assert_eq!(
            read_to_string(&stone).unwrap(),
            "{\"parent\":\"cube\"}",
            "from the bundle"
        );
        assert!(is_file(&stone));
        let mut names: Vec<_> = read_dir(root.join("assets/minecraft/models"))
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        assert_eq!(names, ["block", "item"]);
        // A directory with a loose file is listed from the disk, so the sound is there.
        let sounds: Vec<_> = read_dir(root.join("assets/minecraft/sounds"))
            .unwrap()
            .collect();
        assert_eq!(sounds.len(), 1);
        // Missing from a directory the bundle holds whole: not found, without the disk.
        let missing = root.join("assets/minecraft/models/block/stone.png.mcmeta");
        std::fs::write(&missing, b"on disk after the bundle").unwrap();
        assert_eq!(read(&missing).unwrap_err().kind(), io::ErrorKind::NotFound);
        // Outside the bundle's trees: the disk.
        assert!(read(root.join(KEY_FILE)).is_ok());
        unmount_all();
    }

    #[test]
    fn a_file_added_after_the_bundle_makes_it_stale() {
        let s = Scratch::new("added");
        let root = &s.0;
        put(root, KEY_FILE, b"{}");
        put(root, "assets/minecraft/lang/en_us.json", b"{}");
        let bytes = build(root, 1).unwrap();
        let before = header_tree_key(&bytes);
        assert_eq!(before, tree_key(&parse(root, bytes.clone()).unwrap()));
        std::thread::sleep(std::time::Duration::from_millis(20));
        put(root, "assets/minecraft/lang/de_de.json", b"{}");
        assert_ne!(
            before,
            tree_key(&parse(root, bytes).unwrap()),
            "the lang folder changed"
        );
    }

    #[test]
    fn a_stale_or_broken_bundle_is_never_used() {
        assert_eq!(header_key(b"CLBN"), None);
        assert_eq!(header_key(b"nope and more bytes here"), None);
        let s = Scratch::new("stale");
        put(&s.0, KEY_FILE, b"{}");
        put(&s.0, "data/a.json", b"1");
        let mut bytes = build(&s.0, 1).unwrap();
        bytes.truncate(bytes.len() - 1);
        assert!(parse(&s.0, bytes).is_none(), "cut short");
    }
}
