//! Finding the project's input assets while the game runs, and noticing when
//! they change on disk.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::library::{project_root, Library, MappingEntry};
use crate::model::AssetId;

/// Look again for a mapping that isn't found at most this often.
const RESCAN_MISSING: Duration = Duration::from_secs(1);

static ROOT_OVERRIDE: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Use `root` as the project, instead of finding it. For hosts that know
/// better than the environment does.
pub fn set_project_root(root: Option<PathBuf>) {
    *ROOT_OVERRIDE.lock().unwrap_or_else(|p| p.into_inner()) = root;
}

/// The project the running game belongs to: the root set with
/// [`set_project_root`], else `PULSAR_PROJECT_ROOT` (which Play-in-Editor
/// sets), else the nearest project around the working directory or the
/// executable.
pub fn find_project_root() -> Option<PathBuf> {
    if let Some(root) = ROOT_OVERRIDE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone()
    {
        return Some(root);
    }
    if let Some(root) = std::env::var_os("PULSAR_PROJECT_ROOT")
        .map(PathBuf::from)
        .filter(|p| engine_fs::virtual_fs::is_dir(p))
    {
        return Some(root);
    }
    let is_project = |dir: &Path| {
        engine_fs::virtual_fs::is_file(&dir.join("Pulsar.toml"))
            || engine_fs::virtual_fs::is_dir(&dir.join(".pulsar"))
    };
    let from_cwd = std::env::current_dir().ok().and_then(|cwd| {
        cwd.ancestors()
            .find(|d| is_project(d))
            .map(Path::to_path_buf)
    });
    from_cwd.or_else(|| {
        let exe = std::env::current_exe().ok()?;
        exe.ancestors()
            .skip(1)
            .find(|d| is_project(d))
            .map(Path::to_path_buf)
    })
}

/// Files and when they were last written, to tell when one changed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stamps(Vec<(PathBuf, Option<u128>)>);

impl Stamps {
    pub fn of(paths: impl IntoIterator<Item = PathBuf>) -> Self {
        Self(
            paths
                .into_iter()
                .map(|p| {
                    let at = engine_fs::virtual_fs::metadata(&p)
                        .ok()
                        .and_then(|m| m.modified_nanos());
                    (p, at)
                })
                .collect(),
        )
    }

    /// Whether any of the files was written, removed or created since.
    pub fn changed(&self) -> bool {
        self.0.iter().any(|(p, at)| {
            engine_fs::virtual_fs::metadata(p)
                .ok()
                .and_then(|m| m.modified_nanos())
                != *at
        })
    }
}

/// The project's input assets, scanned on first use.
pub struct Assets {
    root: Option<PathBuf>,
    library: Option<Arc<Library>>,
    scanned: Option<Instant>,
    /// Scans run on a thread of their own (the game's tick must not read the
    /// project); `None`: they run where they are asked for.
    background: bool,
    /// A scan in flight, and when it was asked for.
    scan: Option<std::sync::mpsc::Receiver<Library>>,
}

impl Assets {
    /// `root` is the project; `None` finds it.
    pub fn new(root: Option<PathBuf>) -> Self {
        Self {
            root: root.or_else(find_project_root),
            library: None,
            scanned: None,
            background: false,
            scan: None,
        }
    }

    /// Scan the project on a thread of its own from now on: [`Self::library`]
    /// answers an empty library until the scan is in ([`Self::scanning`]), and
    /// a mapping looked for meanwhile is not found yet, to be asked for again.
    /// The game's tick, which runs with the world locked, uses it: reading a
    /// whole project there held up every frame for tens of milliseconds.
    pub fn scan_in_background(&mut self) {
        self.background = true;
    }

    /// Whether scans run on a thread of their own.
    pub fn is_background(&self) -> bool {
        self.background
    }

    /// A scan is running and no library is in yet.
    pub fn scanning(&self) -> bool {
        self.scan.is_some() && self.library.is_none()
    }

    fn start_scan(&mut self) {
        let Some(root) = self.root.clone() else {
            self.library = Some(Arc::new(Library::empty(PathBuf::new())));
            return;
        };
        let (send, receive) = std::sync::mpsc::channel();
        let started = editor_task_queue::spawn_thread(
            editor_task_queue::ThreadSpec::new("pulsar-input-scan", editor_task_queue::ThreadRole::Io)
                .owner("input_plugin"),
            move || {
                profiling::profile_scope!("Input::scan_project");
                let _ = send.send(Library::scan(&root));
            },
        );
        match started {
            Ok(_) => self.scan = Some(receive),
            Err(error) => {
                tracing::warn!(%error, "input: the project scan could not start a thread; scanning here");
                self.rescan();
            }
        }
    }

    /// Take the background scan's result if it is in.
    fn poll_scan(&mut self) {
        let Some(receive) = &self.scan else {
            return;
        };
        match receive.try_recv() {
            Ok(library) => {
                self.library = Some(Arc::new(library));
                self.scanned = Some(Instant::now());
                self.scan = None;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(std::sync::mpsc::TryRecvError::Disconnected) => self.scan = None,
        }
    }

    pub fn root(&self) -> Option<&Path> {
        self.root.as_deref()
    }

    /// The library, scanning the project the first time.
    pub fn library(&mut self) -> Arc<Library> {
        self.poll_scan();
        if self.library.is_none() && self.scan.is_none() {
            // The session start scanned the project already.
            let prescanned = self.root.as_deref().and_then(crate::library::take_prescanned);
            match prescanned {
                Some(library) => {
                    self.scanned = Some(Instant::now());
                    self.library = Some(library);
                }
                None if self.background => self.start_scan(),
                None => self.rescan(),
            }
        }
        self.library
            .clone()
            .unwrap_or_else(|| Arc::new(Library::empty(PathBuf::new())))
    }

    /// Read the project again.
    pub fn rescan(&mut self) {
        profiling::profile_scope!("Input::scan_project");
        self.scanned = Some(Instant::now());
        self.library = Some(Arc::new(match &self.root {
            Some(root) => Library::scan(root),
            None => Library::empty(PathBuf::new()),
        }));
    }

    /// The mapping `key` names: its project-relative path, a path to the
    /// file, or its id. A mapping that isn't found triggers a fresh scan, at
    /// most once a second, in case it was just made.
    pub fn mapping(&mut self, key: &str) -> Option<MappingEntry> {
        let root = self.root.clone();
        let find = |lib: &Library| -> Option<MappingEntry> {
            if let Some(id) = AssetId::parse(key) {
                return lib.mapping_entry(&id).cloned();
            }
            let relative = match &root {
                Some(root) => crate::library::relative_path(root, Path::new(key)),
                None => key.to_string(),
            };
            lib.mapping_at_relative(&relative).cloned()
        };
        let library = self.library();
        if let Some(found) = find(&library) {
            return Some(found);
        }
        if self.scanning() || self.scanned.is_some_and(|at| at.elapsed() < RESCAN_MISSING) {
            return None;
        }
        if self.background {
            // Look again from a fresh read of the project, once it is in (the old
            // library stays in use until then).
            self.scanned = Some(Instant::now());
            self.start_scan();
            return None;
        }
        self.rescan();
        let library = self.library();
        find(&library)
    }

    /// The project root a mapping's library paths are relative to.
    pub fn resolve_root(file: &Path) -> PathBuf {
        project_root(file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Asset, MappingAsset};

    fn dir(name: &str) -> PathBuf {
        let d =
            std::env::temp_dir().join(format!("input_plugin_assets_{}_{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn a_mapping_is_found_by_relative_path_absolute_path_or_id() {
        let root = dir("find");
        let map = MappingAsset::new();
        std::fs::create_dir_all(root.join("input")).unwrap();
        std::fs::write(root.join("input/Game.inputmap"), map.to_text()).unwrap();
        let mut assets = Assets::new(Some(root.clone()));
        assert_eq!(assets.mapping("input/Game.inputmap").unwrap().id, map.id);
        assert_eq!(assets.mapping(r"input\Game.inputmap").unwrap().id, map.id);
        assert_eq!(
            assets
                .mapping(root.join("input/Game.inputmap").to_str().unwrap())
                .unwrap()
                .id,
            map.id
        );
        assert_eq!(assets.mapping(map.id.as_str()).unwrap().id, map.id);
        assert!(assets.mapping("nope.inputmap").is_none());
        std::fs::remove_dir_all(&root).ok();
    }

    /// The tick must never read the project: a background scan answers "not found
    /// yet" at once, and the mapping is there once the scan is in.
    #[test]
    fn a_background_scan_does_not_hold_the_caller_and_the_mapping_is_found_once_it_is_in() {
        let root = dir("background");
        let map = MappingAsset::new();
        std::fs::create_dir_all(root.join("input")).unwrap();
        std::fs::write(root.join("input/Game.inputmap"), map.to_text()).unwrap();
        let mut assets = Assets::new(Some(root.clone()));
        assets.scan_in_background();
        assert!(assets.mapping("input/Game.inputmap").is_none(), "answered while scanning");
        let deadline = Instant::now() + Duration::from_secs(10);
        let found = loop {
            if let Some(found) = assets.mapping("input/Game.inputmap") {
                break found;
            }
            assert!(Instant::now() < deadline, "the scan never came in");
            std::thread::sleep(Duration::from_millis(5));
        };
        assert_eq!(found.id, map.id);
        assert!(!assets.scanning());
        std::fs::remove_dir_all(&root).ok();
    }

    /// The scan a session start made is the first frame's library, once.
    #[test]
    fn the_session_starts_scan_is_taken_by_the_first_frame_once() {
        let root = dir("prescan");
        let map = MappingAsset::new();
        std::fs::write(root.join("Game.inputmap"), map.to_text()).unwrap();
        crate::library::prescan(&root);
        // Made after the scan: only a fresh scan could find it.
        let late = MappingAsset::new();
        std::fs::write(root.join("Late.inputmap"), late.to_text()).unwrap();
        let mut assets = Assets::new(Some(root.clone()));
        assert!(assets.mapping("Game.inputmap").is_some());
        assert!(
            assets.library().mapping_entry(&late.id).is_none(),
            "the first frame used the session start's scan, not a new one"
        );
        assert!(crate::library::take_prescanned(&root).is_none(), "taken once");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a_mapping_made_after_the_first_scan_is_found_on_a_later_try() {
        let root = dir("late");
        let mut assets = Assets::new(Some(root.clone()));
        assert!(assets.mapping("Late.inputmap").is_none());
        let map = MappingAsset::new();
        std::fs::write(root.join("Late.inputmap"), map.to_text()).unwrap();
        assert!(
            assets.mapping("Late.inputmap").is_none(),
            "scans aren't repeated every frame"
        );
        assets.scanned = Some(Instant::now() - RESCAN_MISSING * 2);
        assert_eq!(assets.mapping("Late.inputmap").unwrap().id, map.id);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn stamps_notice_a_write_a_removal_and_a_creation() {
        let root = dir("stamps");
        let file = root.join("a.txt");
        let stamps = Stamps::of([file.clone()]);
        assert!(!stamps.changed());
        std::fs::write(&file, "x").unwrap();
        assert!(stamps.changed(), "created");
        let stamps = Stamps::of([file.clone()]);
        assert!(!stamps.changed());
        std::fs::remove_file(&file).unwrap();
        assert!(stamps.changed(), "removed");
        std::fs::remove_dir_all(&root).ok();
    }
}
