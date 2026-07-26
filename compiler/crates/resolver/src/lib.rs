use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ModuleInfo {
    pub path: Vec<String>,
    pub file_path: PathBuf,
    pub import_name: String,
}

#[derive(Debug)]
pub enum ResolveError {
    NotFound {
        import_path: Vec<String>,
        searched: Vec<PathBuf>,
    },
    Io(std::io::Error),
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolveError::NotFound {
                import_path,
                searched,
            } => {
                write!(f, "module `{}` not found", import_path.join("::"))?;
                for path in searched {
                    write!(f, "\n  searched: {}", path.display())?;
                }
                Ok(())
            }
            ResolveError::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl std::error::Error for ResolveError {}

#[derive(Debug, Clone)]
pub struct ModuleResolver {
    source_root: PathBuf,
    modules: HashMap<Vec<String>, ModuleInfo>,
}

impl ModuleResolver {
    pub fn new(source_root: &Path) -> Result<Self, ResolveError> {
        let mut modules = HashMap::new();
        let source_root = source_root.canonicalize().map_err(ResolveError::Io)?;
        collect_modules(&source_root, &source_root, &mut modules).map_err(ResolveError::Io)?;
        Ok(ModuleResolver {
            source_root,
            modules,
        })
    }

    pub fn resolve(&self, import_path: &[String]) -> Result<&ModuleInfo, ResolveError> {
        if let Some(info) = self.modules.get(import_path) {
            return Ok(info);
        }
        let mut searched = Vec::new();
        let ext = "vn";
        let file_path = self.module_file_path(import_path, ext);
        searched.push(file_path.clone());
        if file_path.exists() {
            unreachable!(
                "file exists but was not discovered by walk: {}",
                file_path.display()
            );
        }
        Err(ResolveError::NotFound {
            import_path: import_path.to_vec(),
            searched,
        })
    }

    fn module_file_path(&self, import_path: &[String], ext: &str) -> PathBuf {
        let mut path = self.source_root.clone();
        for segment in import_path {
            path.push(segment);
        }
        path.set_extension(ext);
        path
    }

    pub fn module_for_path(&self, file_path: &Path) -> Option<&ModuleInfo> {
        let canonical = file_path.canonicalize().ok()?;
        let relative = canonical.strip_prefix(&self.source_root).ok()?;
        let mut parts: Vec<String> = relative
            .iter()
            .map(|s| s.to_string_lossy().to_string())
            .collect();
        if let Some(last) = parts.last_mut()
            && let Some(stem) = last.rsplit_once('.')
        {
            *last = stem.0.to_string();
        }
        if parts.len() >= 2 && parts.last() == parts.get(parts.len() - 2) {
            parts.pop();
        }
        self.modules.get(&parts)
    }

    pub fn all_modules(&self) -> &HashMap<Vec<String>, ModuleInfo> {
        &self.modules
    }

    pub fn source_root(&self) -> &Path {
        &self.source_root
    }

    pub fn add_module(&mut self, path: &Path) -> Result<(), ResolveError> {
        let canonical = path.canonicalize().map_err(ResolveError::Io)?;
        if canonical.is_dir() {
            collect_modules(&canonical, &self.source_root, &mut self.modules)
                .map_err(ResolveError::Io)?;
        } else {
            add_module_path(&canonical, &self.source_root, &mut self.modules)
                .map_err(ResolveError::Io)?;
        }
        Ok(())
    }

    pub fn remove_module(&mut self, path: &Path) -> bool {
        let canonical = path.canonicalize().ok();
        let len_before = self.modules.len();
        self.modules.retain(|_, info| {
            if let Some(ref canonical) = canonical {
                info.file_path.as_path() != canonical.as_path()
            } else {
                info.file_path != path
            }
        });
        self.modules.len() != len_before
    }
}

fn add_module_path(
    path: &Path,
    source_root: &Path,
    modules: &mut HashMap<Vec<String>, ModuleInfo>,
) -> std::io::Result<()> {
    if path.extension().is_none_or(|e| e != "vn") {
        return Ok(());
    }

    let file_stem = path.file_stem().unwrap().to_string_lossy().to_string();
    let relative = path.strip_prefix(source_root).unwrap_or(path);
    let mut parts: Vec<String> = relative
        .iter()
        .map(|s| {
            let s = s.to_string_lossy().to_string();
            if let Some(stem) = s.rsplit_once('.') {
                stem.0.to_string()
            } else {
                s
            }
        })
        .collect();

    if parts.len() >= 2 && parts.last() == parts.get(parts.len() - 2) {
        parts.pop();
    }

    let parent_dir_name = path
        .parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let is_dir_module = file_stem == parent_dir_name;
    let import_name = parts.last().cloned().unwrap_or(file_stem);

    let info = ModuleInfo {
        path: parts.clone(),
        file_path: path.to_path_buf(),
        import_name,
    };

    if is_dir_module {
        modules.entry(parts).or_insert(info);
    } else {
        modules.insert(parts, info);
    }

    Ok(())
}

fn collect_modules(
    dir: &Path,
    source_root: &Path,
    modules: &mut HashMap<Vec<String>, ModuleInfo>,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_modules(&path, source_root, modules)?;
        } else {
            add_module_path(&path, source_root, modules)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_test_dir(name: &str, files: &[&str]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vinyl_resolver_test_{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        for file in files {
            let path = dir.join(file);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&path, "").unwrap();
        }
        dir
    }

    #[test]
    fn finds_root_file() {
        let dir = setup_test_dir("root_file", &["test.vn"]);
        let resolver = ModuleResolver::new(&dir).unwrap();
        let info = resolver.resolve(&["test".to_string()]).unwrap();
        assert_eq!(info.import_name, "test");
        assert_eq!(info.path, vec!["test"]);
    }

    #[test]
    fn finds_directory_module() {
        let dir = setup_test_dir("dir_module", &["test/test.vn"]);
        let resolver = ModuleResolver::new(&dir).unwrap();
        let info = resolver.resolve(&["test".to_string()]).unwrap();
        assert_eq!(info.import_name, "test");
        assert_eq!(info.path, vec!["test"]);
    }

    #[test]
    fn finds_nested_file() {
        let dir = setup_test_dir("nested", &["test/stats.vn"]);
        let resolver = ModuleResolver::new(&dir).unwrap();
        let info = resolver
            .resolve(&["test".to_string(), "stats".to_string()])
            .unwrap();
        assert_eq!(info.import_name, "stats");
        assert_eq!(info.path, vec!["test", "stats"]);
    }

    #[test]
    fn not_found_error() {
        let dir = setup_test_dir("not_found", &["foo.vn"]);
        let resolver = ModuleResolver::new(&dir).unwrap();
        let err = resolver.resolve(&["nonexistent".to_string()]).unwrap_err();
        assert!(matches!(err, ResolveError::NotFound { .. }));
    }

    #[test]
    fn module_for_path_lookup() {
        let dir = setup_test_dir("path_lookup", &["test/stats.vn"]);
        let resolver = ModuleResolver::new(&dir).unwrap();
        let info = resolver.module_for_path(&dir.join("test/stats.vn"));
        assert!(info.is_some());
        assert_eq!(info.unwrap().import_name, "stats");
    }

    #[test]
    fn empty_source_root() {
        let dir = setup_test_dir("empty", &[]);
        let resolver = ModuleResolver::new(&dir).unwrap();
        assert!(resolver.all_modules().is_empty());
    }

    #[test]
    fn prefers_file_over_directory() {
        let dir = setup_test_dir("prefer_file", &["test.vn", "test/test.vn"]);
        let resolver = ModuleResolver::new(&dir).unwrap();
        let info = resolver.resolve(&["test".to_string()]).unwrap();
        assert_eq!(
            info.file_path.file_name().unwrap(),
            "test.vn",
            "should prefer test.vn over test/test.vn"
        );
    }

    #[test]
    fn multiple_files_in_dir() {
        let dir = setup_test_dir("multi_file", &["foo.vn", "bar.vn", "baz/qux.vn"]);
        let resolver = ModuleResolver::new(&dir).unwrap();
        assert!(resolver.resolve(&["foo".to_string()]).is_ok());
        assert!(resolver.resolve(&["bar".to_string()]).is_ok());
        assert!(
            resolver
                .resolve(&["baz".to_string(), "qux".to_string()])
                .is_ok()
        );
    }

    #[test]
    fn ignores_non_vinyl_files() {
        let dir = setup_test_dir("ignore_other", &["test.vn", "readme.md", "data.json"]);
        let resolver = ModuleResolver::new(&dir).unwrap();
        assert_eq!(resolver.all_modules().len(), 1);
        assert!(resolver.resolve(&["test".to_string()]).is_ok());
    }

    #[test]
    fn subdirectory_prefixed_module() {
        let dir = setup_test_dir("subdir_prefixed", &["app/routes/users.vn"]);
        let resolver = ModuleResolver::new(&dir).unwrap();
        let info = resolver
            .resolve(&["app".to_string(), "routes".to_string(), "users".to_string()])
            .unwrap();
        assert_eq!(info.import_name, "users");
    }

    #[test]
    fn directory_module_dedup() {
        let dir = setup_test_dir("dedup", &["foo/bar/bar.vn"]);
        let resolver = ModuleResolver::new(&dir).unwrap();
        let info = resolver
            .resolve(&["foo".to_string(), "bar".to_string()])
            .unwrap();
        assert_eq!(info.import_name, "bar");
        assert_eq!(info.path, vec!["foo", "bar"]);
    }

    #[test]
    fn deep_nested_file() {
        let dir = setup_test_dir("deep_nested", &["a/b/c/d.vn"]);
        let resolver = ModuleResolver::new(&dir).unwrap();
        let info = resolver
            .resolve(&[
                "a".to_string(),
                "b".to_string(),
                "c".to_string(),
                "d".to_string(),
            ])
            .unwrap();
        assert_eq!(info.import_name, "d");
    }

    #[test]
    fn add_module_single_file() {
        let dir = setup_test_dir("add_single", &["existing.vn"]);
        let mut resolver = ModuleResolver::new(&dir).unwrap();
        assert_eq!(resolver.all_modules().len(), 1);

        let new_file = dir.join("new_module.vn");
        fs::write(&new_file, "").unwrap();
        resolver.add_module(&new_file).unwrap();
        assert_eq!(resolver.all_modules().len(), 2);
        assert!(resolver.resolve(&["new_module".to_string()]).is_ok());
    }

    #[test]
    fn add_module_directory() {
        let dir = setup_test_dir("add_dir", &["existing.vn"]);
        let mut resolver = ModuleResolver::new(&dir).unwrap();
        assert_eq!(resolver.all_modules().len(), 1);

        let sub_dir = dir.join("sub");
        fs::create_dir_all(&sub_dir).unwrap();
        fs::write(sub_dir.join("a.vn"), "").unwrap();
        fs::write(sub_dir.join("b.vn"), "").unwrap();
        resolver.add_module(&sub_dir).unwrap();
        assert_eq!(resolver.all_modules().len(), 3);
    }

    #[test]
    fn remove_module_by_path() {
        let dir = setup_test_dir("remove_by_path", &["foo.vn", "bar.vn"]);
        let mut resolver = ModuleResolver::new(&dir).unwrap();
        assert_eq!(resolver.all_modules().len(), 2);

        let removed = resolver.remove_module(&dir.join("foo.vn"));
        assert!(removed);
        assert_eq!(resolver.all_modules().len(), 1);
        assert!(resolver.resolve(&["bar".to_string()]).is_ok());
    }

    #[test]
    fn remove_module_non_existent() {
        let dir = setup_test_dir("remove_missing", &["foo.vn"]);
        let mut resolver = ModuleResolver::new(&dir).unwrap();
        let removed = resolver.remove_module(&dir.join("nonexistent.vn"));
        assert!(!removed);
        assert_eq!(resolver.all_modules().len(), 1);
    }
}
