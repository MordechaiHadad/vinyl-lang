use std::collections::HashMap;
use std::fmt::Debug;
use std::path::{Path, PathBuf};

pub mod error;
pub use error::ResolveDiagnostic;

#[derive(Debug, Clone)]
pub struct ModuleInfo {
    pub path: Vec<String>,
    pub file_path: PathBuf,
    pub import_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportPrefix {
    Self_,
    Parent(usize),
    Package,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolverMode {
    Manifest,
    Script,
}

pub trait FileSystem: Debug + Send + Sync {
    fn file_exists(&self, path: &Path) -> bool;
    fn collect_vn_files(&self, dir: &Path) -> Result<Vec<PathBuf>, ResolveDiagnostic>;
}

#[derive(Debug)]
pub struct DiskFileSystem;

impl FileSystem for DiskFileSystem {
    fn file_exists(&self, path: &Path) -> bool {
        path.is_file()
    }

    fn collect_vn_files(&self, dir: &Path) -> Result<Vec<PathBuf>, ResolveDiagnostic> {
        let mut files = Vec::new();
        let walker = ignore::WalkBuilder::new(dir)
            .parents(true)
            .git_ignore(true)
            .git_global(true)
            .require_git(false)
            .build();
        for result in walker {
            let entry = result.map_err(|e| ResolveDiagnostic::Io(std::io::Error::other(e)))?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "vn") {
                files.push(path.to_path_buf());
            }
        }
        files.sort();
        Ok(files)
    }
}

#[derive(Debug)]
pub struct Resolver {
    mode: ResolverMode,
    root: PathBuf,
    modules: HashMap<Vec<String>, ModuleInfo>,
    fs: Box<dyn FileSystem>,
}

impl Resolver {
    // -- Convenience constructors (DiskFileSystem) --

    pub fn detect(entry: &Path) -> Result<Self, ResolveDiagnostic> {
        Self::detect_with(entry, Box::new(DiskFileSystem))
    }

    pub fn for_manifest(root: &Path) -> Result<Self, ResolveDiagnostic> {
        Self::for_manifest_with(root, Box::new(DiskFileSystem))
    }

    pub fn for_script(root: &Path) -> Self {
        Self::for_script_with(root, Box::new(DiskFileSystem))
    }

    // -- Custom filesystem constructors --

    pub fn detect_with(entry: &Path, fs: Box<dyn FileSystem>) -> Result<Self, ResolveDiagnostic> {
        let entry = std::path::absolute(entry)?;

        if entry.is_file() && entry.extension().is_some_and(|e| e != "vn") {
            return Err(ResolveDiagnostic::NotFound {
                import_path: vec![
                    entry
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                ],
                searched: vec![entry],
            });
        }

        let start_dir = if entry.is_dir() {
            entry.clone()
        } else {
            entry.parent().unwrap_or(Path::new("")).to_path_buf()
        };

        if let Some(root) = find_manifest_dir(&start_dir) {
            Self::for_manifest_with(&root, fs)
        } else {
            let root = if entry.is_dir() {
                entry
            } else {
                entry.parent().unwrap_or(Path::new("")).to_path_buf()
            };
            Ok(Self::for_script_with(&root, fs))
        }
    }

    pub fn for_manifest_with(
        root: &Path,
        fs: Box<dyn FileSystem>,
    ) -> Result<Self, ResolveDiagnostic> {
        let root = std::path::absolute(root)?;
        let src = root.join("src");
        if !src.is_dir() {
            return Err(ResolveDiagnostic::MissingSrcDir { root });
        }

        let files = fs.collect_vn_files(&src)?;
        let mut modules = HashMap::new();
        for file_path in &files {
            add_module_path(file_path, &src, &mut modules);
        }

        Ok(Resolver {
            mode: ResolverMode::Manifest,
            root,
            modules,
            fs,
        })
    }

    pub fn for_script_with(root: &Path, fs: Box<dyn FileSystem>) -> Self {
        Resolver {
            mode: ResolverMode::Script,
            root: root.to_path_buf(),
            modules: HashMap::new(),
            fs,
        }
    }

    // -- Accessors --

    pub fn mode(&self) -> &ResolverMode {
        &self.mode
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn all_modules(&self) -> &HashMap<Vec<String>, ModuleInfo> {
        &self.modules
    }

    pub fn list_vn_files(&self, dir: &Path) -> Result<Vec<PathBuf>, ResolveDiagnostic> {
        self.fs.collect_vn_files(dir)
    }

    pub fn register_module(&mut self, file_path: &Path) {
        let source_root = match self.mode {
            ResolverMode::Manifest => self.root.join("src"),
            ResolverMode::Script => self.root.clone(),
        };
        add_module_path(file_path, &source_root, &mut self.modules);
    }

    pub fn resolve_module_path(
        &mut self,
        path: &[String],
    ) -> Result<ModuleInfo, ResolveDiagnostic> {
        let import_path: Vec<String> = path.to_vec();
        if let Some(info) = self.modules.get(path) {
            return Ok(info.clone());
        }
        if let ResolverMode::Script = &self.mode {
            let mut file_path = self.root.clone();
            for seg in path {
                file_path.push(seg);
            }
            file_path.set_extension("vn");
            let normalized = normalize_path(&file_path);
            if self.fs.file_exists(&normalized) {
                let relative = normalized
                    .strip_prefix(&self.root)
                    .unwrap_or(&normalized)
                    .to_path_buf();
                let module_path = path_from_relative(&relative);
                let import_name = path.last().unwrap_or(&"".to_string()).clone();
                let info = ModuleInfo {
                    path: module_path.clone(),
                    file_path: normalized,
                    import_name,
                };
                return Ok(self.modules.entry(module_path).or_insert(info).clone());
            }
            return Err(ResolveDiagnostic::NotFound {
                import_path,
                searched: vec![normalized],
            });
        }
        let searched = {
            let mut p = self.root.join("src");
            for seg in path {
                p.push(seg);
            }
            p.set_extension("vn");
            vec![p]
        };
        Err(ResolveDiagnostic::NotFound {
            import_path,
            searched,
        })
    }

    pub fn resolve(
        &mut self,
        prefix: &ImportPrefix,
        path: &[&str],
        from: &Path,
    ) -> Result<ModuleInfo, ResolveDiagnostic> {
        match self.mode {
            ResolverMode::Manifest => self.resolve_manifest(prefix, path, from),
            ResolverMode::Script => self.resolve_script(prefix, path, from),
        }
    }

    fn resolve_manifest(
        &self,
        prefix: &ImportPrefix,
        path: &[&str],
        from: &Path,
    ) -> Result<ModuleInfo, ResolveDiagnostic> {
        match prefix {
            ImportPrefix::Package => {
                let module_path: Vec<String> = path.iter().map(|s| s.to_string()).collect();
                self.modules.get(&module_path).cloned().ok_or_else(|| {
                    let searched = vec![self.module_absolute_path(&module_path)];
                    ResolveDiagnostic::NotFound {
                        import_path: module_path,
                        searched,
                    }
                })
            }
            ImportPrefix::Self_ | ImportPrefix::Parent(_) => {
                let target = compute_target_path(prefix, path, from);
                let normalized_target = normalize_path(&target);
                let src = self.root.join("src");
                let normalized_src = normalize_path(&src);

                if !normalized_target.starts_with(&normalized_src) {
                    return Err(ResolveDiagnostic::AboveRoot {
                        import_path: path.iter().map(|s| s.to_string()).collect(),
                    });
                }

                let relative = normalized_target
                    .strip_prefix(&normalized_src)
                    .unwrap()
                    .to_path_buf();
                let mut module_path = path_from_relative(&relative);

                if module_path.len() >= 2
                    && module_path.last() == module_path.get(module_path.len() - 2)
                {
                    module_path.pop();
                }

                self.modules
                    .get(&module_path)
                    .cloned()
                    .ok_or_else(|| ResolveDiagnostic::NotFound {
                        import_path: module_path,
                        searched: vec![target],
                    })
            }
        }
    }

    fn resolve_script(
        &mut self,
        prefix: &ImportPrefix,
        path: &[&str],
        from: &Path,
    ) -> Result<ModuleInfo, ResolveDiagnostic> {
        match prefix {
            ImportPrefix::Package => Err(ResolveDiagnostic::InvalidPrefix {
                prefix: "package".to_string(),
                mode: "script".to_string(),
            }),
            ImportPrefix::Self_ | ImportPrefix::Parent(_) => {
                let target = compute_target_path(prefix, path, from);
                let import_path: Vec<String> = path.iter().map(|s| s.to_string()).collect();

                if !self.fs.file_exists(&target) {
                    return Err(ResolveDiagnostic::NotFound {
                        import_path,
                        searched: vec![target],
                    });
                }

                let relative = target
                    .strip_prefix(&self.root)
                    .unwrap_or(&target)
                    .to_path_buf();
                let module_path = path_from_relative(&relative);
                let import_name = path.last().unwrap_or(&"").to_string();

                let info = ModuleInfo {
                    path: module_path.clone(),
                    file_path: normalize_path(&target),
                    import_name,
                };

                Ok(self.modules.entry(module_path).or_insert(info).clone())
            }
        }
    }

    fn module_absolute_path(&self, module_path: &[String]) -> PathBuf {
        let mut path = self.root.join("src");
        for segment in module_path {
            path.push(segment);
        }
        path.set_extension("vn");
        path
    }
}

fn find_manifest_dir(start: &Path) -> Option<PathBuf> {
    let mut dir = Some(start.to_path_buf());
    while let Some(ref current) = dir {
        if current.join("vinyl.toml").is_file() {
            return Some(current.clone());
        }
        dir = current.parent().map(|p| p.to_path_buf());
    }
    None
}

fn compute_target_path(prefix: &ImportPrefix, path: &[&str], from: &Path) -> PathBuf {
    let mut base = from.parent().unwrap_or(Path::new("")).to_path_buf();

    if let ImportPrefix::Parent(n) = prefix {
        for _ in 0..*n {
            base.push("..");
        }
    }

    for segment in path {
        base.push(segment);
    }
    base.set_extension("vn");
    base
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                result.pop();
            }
            other => {
                result.push(other.as_os_str());
            }
        }
    }
    result
}

fn path_from_relative(relative: &Path) -> Vec<String> {
    relative
        .iter()
        .map(|s| {
            let s = s.to_string_lossy().to_string();
            if let Some(stem) = s.rsplit_once('.') {
                stem.0.to_string()
            } else {
                s
            }
        })
        .collect()
}

fn add_module_path(
    file_path: &Path,
    source_root: &Path,
    modules: &mut HashMap<Vec<String>, ModuleInfo>,
) {
    if file_path.extension().is_none_or(|e| e != "vn") {
        return;
    }

    let file_stem = file_path.file_stem().unwrap().to_string_lossy().to_string();
    let relative = file_path.strip_prefix(source_root).unwrap_or(file_path);
    let mut parts = path_from_relative(relative);

    if parts.len() >= 2 && parts.last() == parts.get(parts.len() - 2) {
        parts.pop();
    }

    let parent_dir_name = file_path
        .parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let is_dir_module = file_stem == parent_dir_name;
    let import_name = parts.last().cloned().unwrap_or(file_stem);

    let info = ModuleInfo {
        path: parts.clone(),
        file_path: file_path.to_path_buf(),
        import_name,
    };

    if is_dir_module {
        modules.entry(parts).or_insert(info);
    } else {
        modules.insert(parts, info);
    }
}
