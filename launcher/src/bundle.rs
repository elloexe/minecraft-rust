//! The game shipped beside the launcher.
//!
//! The Windows build is one folder: `Claudium-Launcher.exe`, the game in `game/` (the
//! `claudium-client` binary, its tools, `research-lock.json` and the precompiled exporters) and
//! a Java runtime in `runtime/`. There is no release to download, so on start the launcher adopts
//! that folder as an installed version and lets the preparer use that Java. Everything after that
//! is the ordinary launcher: Play prepares once (the only slow step, a few minutes the very first
//! time) and then starts the game directly on every later Play.
//!
//! Nothing here runs when there is no such folder, so a launcher that downloads its versions
//! (upstream's way) behaves exactly as before.

use std::fs;
use std::path::{Path, PathBuf};

use crate::backend;

/// The game folder beside the launcher.
pub const GAME_DIR: &str = "game";
/// The Java runtime beside the launcher.
pub const RUNTIME_DIR: &str = "runtime";
/// Records which bundle a version was installed from, so a newer launcher's game replaces it.
const STAMP: &str = ".bundle-stamp";

fn java_binary() -> &'static str {
    if cfg!(windows) {
        "java.exe"
    } else {
        "java"
    }
}

/// The folder the launcher runs from.
fn exe_dir() -> Option<PathBuf> {
    backend::exe_path().parent().map(Path::to_path_buf)
}

/// The bundled game folder in `base`, if it has the game.
pub fn game_in(base: &Path) -> Option<PathBuf> {
    let dir = base.join(GAME_DIR);
    dir.join(backend::CLIENT).is_file().then_some(dir)
}

/// The bundled Java runtime's home in `base`, if it has `bin/java`.
pub fn runtime_in(base: &Path) -> Option<PathBuf> {
    let dir = base.join(RUNTIME_DIR);
    dir.join("bin").join(java_binary()).is_file().then_some(dir)
}

/// The version tag a bundle installs as: the Minecraft version its research-lock.json pins
/// ("26.3"), else "bundled".
pub fn tag_of(game: &Path) -> String {
    fs::read_to_string(game.join("research-lock.json"))
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|lock| lock["minecraft"]["version"].as_str().map(str::to_string))
        .filter(|v| !v.is_empty() && !v.contains(['/', '\\']) && v != "." && v != "..")
        .unwrap_or_else(|| "bundled".to_string())
}

/// What identifies a bundle: its client binary's size and modification time, and its pins.
/// Cheap to compute on every start (no hashing of a large binary).
pub fn stamp_of(game: &Path) -> String {
    let client = fs::metadata(game.join(backend::CLIENT)).ok();
    let len = client.as_ref().map_or(0, |m| m.len());
    let modified = client
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs());
    let lock = fs::read(game.join("research-lock.json")).unwrap_or_default();
    let lock: u64 = lock.iter().fold(0xcbf29ce484222325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
    });
    format!("{len}:{modified}:{lock:016x}")
}

/// Points Java lookups at the bundled runtime, when there is one and the player has not chosen
/// a Java of their own with JAVA_HOME. Call before any thread starts.
pub fn use_bundled_java() {
    let Some(home) = exe_dir().and_then(|d| runtime_in(&d)) else {
        return;
    };
    if std::env::var_os("JAVA_HOME").is_some_and(|h| !h.is_empty()) {
        return;
    }
    let mut paths = vec![home.join("bin")];
    if let Some(path) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&path));
    }
    std::env::set_var("JAVA_HOME", &home);
    if let Ok(path) = std::env::join_paths(paths) {
        std::env::set_var("PATH", path);
    }
}

/// Installs the game bundled beside the launcher, if there is one, and returns its tag. Cheap
/// when it is installed already: a stat and a small file read.
pub fn adopt() -> Option<Result<String, String>> {
    let game = game_in(&exe_dir()?)?;
    Some(adopt_from(&game))
}

/// Installs `game` (a bundled game folder) as its version, keeping whatever has been prepared.
pub fn adopt_from(game: &Path) -> Result<String, String> {
    let tag = tag_of(game);
    let target = backend::version_dir(&tag);
    let stamp = stamp_of(game);
    if !backend::installed().contains(&tag) {
        adopt_electron_install(&tag);
    }
    let current = fs::read_to_string(target.join(STAMP)).ok();
    if backend::installed().contains(&tag) && current.as_deref() != Some(stamp.as_str()) {
        // A newer launcher with a newer game: copy it over the installed one. This adds and
        // overwrites files, it never deletes, so the prepared resources stay.
        backend::copy_dir(game, &target).map_err(|e| e.to_string())?;
    }
    backend::install_from_dir(&tag, game)?;
    let _ = fs::write(target.join(STAMP), stamp);
    Ok(tag)
}

/// The Electron launcher this one replaced kept its versions in `%APPDATA%\Claudium`. A version
/// prepared there is moved here, so nobody prepares again because the launcher changed.
fn adopt_electron_install(tag: &str) {
    if !cfg!(windows) || std::env::var_os("CLAUDIUM_LAUNCHER_HOME").is_some() {
        return;
    }
    let Some(old) = dirs::data_dir().map(|d| d.join("Claudium").join("versions").join(tag)) else {
        return;
    };
    if !old.join(".installed").exists() {
        return;
    }
    let target = backend::version_dir(tag);
    if target.exists() {
        return;
    }
    if let Some(parent) = target.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::rename(&old, &target);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh scratch folder, removed when dropped.
    struct Scratch(PathBuf);
    impl Scratch {
        fn new(name: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("claudium-bundle-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Scratch(dir)
        }
        fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn write(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    #[test]
    fn finds_the_game_and_runtime_beside_the_launcher() {
        let base = Scratch::new("finds");
        assert_eq!(game_in(base.path()), None);
        assert_eq!(runtime_in(base.path()), None);
        write(&base.path().join(GAME_DIR).join(backend::CLIENT), "bin");
        write(
            &base
                .path()
                .join(RUNTIME_DIR)
                .join("bin")
                .join(java_binary()),
            "java",
        );
        assert_eq!(game_in(base.path()), Some(base.path().join(GAME_DIR)));
        assert_eq!(runtime_in(base.path()), Some(base.path().join(RUNTIME_DIR)));
    }

    #[test]
    fn names_the_version_after_the_pinned_minecraft() {
        let game = Scratch::new("tag");
        assert_eq!(tag_of(game.path()), "bundled");
        write(
            &game.path().join("research-lock.json"),
            r#"{"minecraft":{"version":"26.3"}}"#,
        );
        assert_eq!(tag_of(game.path()), "26.3");
        write(
            &game.path().join("research-lock.json"),
            r#"{"minecraft":{"version":"../evil"}}"#,
        );
        assert_eq!(tag_of(game.path()), "bundled");
    }

    #[test]
    fn a_different_game_has_a_different_stamp() {
        let game = Scratch::new("stamp");
        write(&game.path().join(backend::CLIENT), "one");
        write(&game.path().join("research-lock.json"), "{}");
        let first = stamp_of(game.path());
        assert_eq!(first, stamp_of(game.path()));
        write(&game.path().join("research-lock.json"), r#"{"a":1}"#);
        assert_ne!(first, stamp_of(game.path()));
    }
}
