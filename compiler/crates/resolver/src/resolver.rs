use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::{ResolveDiagnostic, structs::DiskFileSystem, traits::FileSystem};

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

#[derive(Debug)]
pub struct Resolver {
    mode: ResolverMode,
    root: PathBuf,
    modules: HashMap<Vec<String>, ModuleInfo>,
    fs: Box<dyn FileSystem>,
}

fn std_module_info() -> ModuleInfo {
    ModuleInfo {
        path: vec!["std".to_string()],
        file_path: PathBuf::from(vinyl_std::STD_SOURCE_PATH),
        import_name: "std".to_string(),
    }
}

impl Resolver {
    pub fn detect_with(entry: &Path, fs: Box<dyn FileSystem>) -> Result<Self, ResolveDiagnostic> {
        let entry = crate::strip_verbatim_prefix(&std::path::absolute(entry)?);

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

        if let Some(root) = crate::find_manifest_dir(&start_dir) {
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
        let root = crate::strip_verbatim_prefix(&std::path::absolute(root)?);
        let src = root.join("src");
        if !src.is_dir() {
            return Err(ResolveDiagnostic::MissingSrcDir { root });
        }

        let files = fs.collect_vn_files(&src)?;
        let mut modules = HashMap::new();
        for file_path in &files {
            crate::add_module_path(file_path, &src, &mut modules);
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
            root: crate::strip_verbatim_prefix(root),
            modules: HashMap::new(),
            fs,
        }
    }

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
        crate::add_module_path(
            &crate::strip_verbatim_prefix(file_path),
            &source_root,
            &mut self.modules,
        );
    }

    pub fn resolve_module_path(
        &mut self,
        path: &[String],
    ) -> Result<ModuleInfo, ResolveDiagnostic> {
        if path == ["std"] {
            let info = std_module_info();
            return Ok(self
                .modules
                .entry(info.path.clone())
                .or_insert(info)
                .clone());
        }
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
            let normalized = crate::normalize_path(&file_path);
            if self.fs.file_exists(&normalized) {
                let relative = normalized
                    .strip_prefix(&self.root)
                    .unwrap_or(&normalized)
                    .to_path_buf();
                let module_path = crate::path_from_relative(&relative);
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
                let target = crate::compute_target_path(prefix, path, from);
                let normalized_target = crate::normalize_path(&target);
                let src = self.root.join("src");
                let normalized_src = crate::normalize_path(&src);

                if !normalized_target.starts_with(&normalized_src) {
                    return Err(ResolveDiagnostic::AboveRoot {
                        import_path: path.iter().map(|s| s.to_string()).collect(),
                    });
                }

                let relative = normalized_target
                    .strip_prefix(&normalized_src)
                    .unwrap()
                    .to_path_buf();
                let mut module_path = crate::path_from_relative(&relative);

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
                let target = crate::compute_target_path(prefix, path, from);
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
                let module_path = crate::path_from_relative(&relative);
                let import_name = path.last().unwrap_or(&"").to_string();

                let info = ModuleInfo {
                    path: module_path.clone(),
                    file_path: crate::normalize_path(&target),
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

    pub fn detect(entry: &Path) -> Result<Self, ResolveDiagnostic> {
        Self::detect_with(entry, Box::new(DiskFileSystem))
    }

    pub fn for_manifest(root: &Path) -> Result<Self, ResolveDiagnostic> {
        Self::for_manifest_with(root, Box::new(DiskFileSystem))
    }

    pub fn for_script(root: &Path) -> Self {
        Self::for_script_with(root, Box::new(DiskFileSystem))
    }
}
