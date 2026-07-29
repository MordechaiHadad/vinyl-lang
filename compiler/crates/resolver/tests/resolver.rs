use std::{collections::HashMap, fs, path::PathBuf};

use std::path::Path;
use vinyl_resolver::{FileSystem, ImportPrefix, ResolveDiagnostic, Resolver, ResolverMode};

#[derive(Debug)]
struct TestFileSystem(HashMap<PathBuf, String>);

impl FileSystem for TestFileSystem {
    fn file_exists(&self, path: &Path) -> bool {
        path.is_file() || self.0.contains_key(path)
    }
    fn collect_vn_files(&self, _dir: &Path) -> Result<Vec<PathBuf>, ResolveDiagnostic> {
        Ok(self.0.keys().cloned().collect())
    }
}

fn setup_project(name: &str, files: &[&str], gitignore: &[&str]) -> PathBuf {
    let dir = temp_dir(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::write(dir.join("vinyl.toml"), "").unwrap();

    if !gitignore.is_empty() {
        fs::write(dir.join(".gitignore"), gitignore.join("\n")).unwrap();
    }

    for file in files {
        let path = dir.join(file);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, "").unwrap();
    }

    dir
}

fn setup_script_dir(name: &str, files: &[&str]) -> PathBuf {
    let dir = temp_dir(name);
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

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("vinyl_resolver_test_{name}"))
}

#[test]
fn detects_manifest_mode_when_vinyl_toml_in_cwd() {
    let dir = setup_project("manifest_cwd", &["src/main.vn"], &[]);
    let resolver = Resolver::detect(&dir).unwrap();
    assert_eq!(resolver.mode(), &ResolverMode::Manifest);
    assert_eq!(resolver.root(), &dir);
}

#[test]
fn walks_up_parents_to_find_vinyl_toml() {
    let dir = setup_project("walk_up", &["src/main.vn"], &[]);
    let nested = dir.join("a").join("b").join("c");
    fs::create_dir_all(&nested).unwrap();
    let entry = nested.join("file.vn");
    fs::write(&entry, "").unwrap();

    let resolver = Resolver::detect(&entry).unwrap();
    assert_eq!(resolver.mode(), &ResolverMode::Manifest);
    assert_eq!(resolver.root(), &dir);
}

#[test]
fn falls_back_to_script_mode_when_no_vinyl_toml() {
    let dir = setup_script_dir("no_manifest", &["main.vn"]);
    let entry = dir.join("main.vn");
    let resolver = Resolver::detect(&entry).unwrap();
    assert_eq!(resolver.mode(), &ResolverMode::Script);
}

#[test]
fn manifest_mode_errors_without_src_dir() {
    let dir = setup_project("no_src", &[], &[]);
    let err = Resolver::for_manifest(&dir).unwrap_err();
    assert!(matches!(err, ResolveDiagnostic::MissingSrcDir { .. }));
}

#[test]
fn self_prefix_resolves_to_same_directory_file() {
    let dir = setup_project("self_same_dir", &["src/main.vn", "src/helper.vn"], &[]);
    let mut resolver = Resolver::for_manifest(&dir).unwrap();
    let info = resolver
        .resolve(&ImportPrefix::Self_, &["helper"], &dir.join("src/main.vn"))
        .unwrap();
    assert_eq!(info.file_path, dir.join("src/helper.vn"));
    assert_eq!(info.import_name, "helper");
}

#[test]
fn self_prefix_with_nested_path() {
    let dir = setup_project(
        "self_nested",
        &["src/app/main.vn", "src/app/utils/format.vn"],
        &[],
    );
    let mut resolver = Resolver::for_manifest(&dir).unwrap();
    let info = resolver
        .resolve(
            &ImportPrefix::Self_,
            &["utils", "format"],
            &dir.join("src/app/main.vn"),
        )
        .unwrap();
    assert_eq!(info.file_path, dir.join("src/app/utils/format.vn"));
}

#[test]
fn parent_prefix_resolves_to_parent_directory() {
    let dir = setup_project("parent_dir", &["src/foo/child.vn", "src/bar.vn"], &[]);
    let mut resolver = Resolver::for_manifest(&dir).unwrap();
    let info = resolver
        .resolve(
            &ImportPrefix::Parent(1),
            &["bar"],
            &dir.join("src/foo/child.vn"),
        )
        .unwrap();
    assert_eq!(info.file_path, dir.join("src/bar.vn"));
}

#[test]
fn parent_prefix_is_stackable() {
    let dir = setup_project(
        "parent_stack",
        &["src/a/b/c/deep.vn", "src/root_mod.vn"],
        &[],
    );
    let mut resolver = Resolver::for_manifest(&dir).unwrap();
    let info = resolver
        .resolve(
            &ImportPrefix::Parent(3),
            &["root_mod"],
            &dir.join("src/a/b/c/deep.vn"),
        )
        .unwrap();
    assert_eq!(info.file_path, dir.join("src/root_mod.vn"));
}

#[test]
fn parent_prefix_above_root_errors_in_manifest() {
    let dir = setup_project("parent_above", &["src/main.vn"], &[]);
    let mut resolver = Resolver::for_manifest(&dir).unwrap();
    let err = resolver
        .resolve(
            &ImportPrefix::Parent(2),
            &["anything"],
            &dir.join("src/main.vn"),
        )
        .unwrap_err();
    assert!(matches!(err, ResolveDiagnostic::AboveRoot { .. }));
}

#[test]
fn package_prefix_resolves_from_any_file_in_manifest() {
    let dir = setup_project("package_any", &["src/main.vn", "src/net/http.vn"], &[]);
    let mut resolver = Resolver::for_manifest(&dir).unwrap();
    let info = resolver
        .resolve(
            &ImportPrefix::Package,
            &["net", "http"],
            &dir.join("src/main.vn"),
        )
        .unwrap();
    assert_eq!(info.file_path, dir.join("src/net/http.vn"));
}

#[test]
fn package_prefix_rejected_in_script_mode() {
    let dir = setup_script_dir("pkg_rejected", &["main.vn"]);
    let mut resolver = Resolver::for_script(&dir);
    let err = resolver
        .resolve(&ImportPrefix::Package, &["foo"], &dir.join("main.vn"))
        .unwrap_err();
    assert!(matches!(err, ResolveDiagnostic::InvalidPrefix { .. }));
}

#[test]
fn manifest_discovers_all_vn_under_src() {
    let dir = setup_project(
        "discover_all",
        &["src/main.vn", "src/foo.vn", "src/bar/baz.vn"],
        &[],
    );
    let resolver = Resolver::for_manifest(&dir).unwrap();
    assert_eq!(resolver.all_modules().len(), 3);
    assert!(
        resolver
            .all_modules()
            .contains_key(&vec!["main".to_string()])
    );
    assert!(
        resolver
            .all_modules()
            .contains_key(&vec!["foo".to_string()])
    );
    assert!(
        resolver
            .all_modules()
            .contains_key(&vec!["bar".to_string(), "baz".to_string()])
    );
}

#[test]
fn manifest_ignores_files_outside_src() {
    let dir = setup_project(
        "outside_src",
        &["src/main.vn", "README.md", "scripts/build.vn"],
        &[],
    );
    let resolver = Resolver::for_manifest(&dir).unwrap();
    assert_eq!(resolver.all_modules().len(), 1);
    assert!(
        resolver
            .all_modules()
            .contains_key(&vec!["main".to_string()])
    );
}

#[test]
fn manifest_respects_gitignore() {
    let dir = setup_project(
        "gitignored",
        &[
            "src/.gitignore",
            "src/main.vn",
            "src/visible.vn",
            "src/ignored/secret.vn",
        ],
        &[],
    );
    fs::write(dir.join("src/.gitignore"), "ignored/\n").unwrap();
    let resolver = Resolver::for_manifest(&dir).unwrap();
    assert_eq!(resolver.all_modules().len(), 2);
    assert!(
        resolver
            .all_modules()
            .contains_key(&vec!["main".to_string()])
    );
    assert!(
        resolver
            .all_modules()
            .contains_key(&vec!["visible".to_string()])
    );
}

#[test]
fn manifest_ignores_non_vn_files_under_src() {
    let dir = setup_project(
        "non_vn_src",
        &["src/main.vn", "src/helper.rs", "src/data.json"],
        &[],
    );
    let resolver = Resolver::for_manifest(&dir).unwrap();
    assert_eq!(resolver.all_modules().len(), 1);
    assert!(
        resolver
            .all_modules()
            .contains_key(&vec!["main".to_string()])
    );
}

#[test]
fn script_lazy_resolves_self_import() {
    let dir = setup_script_dir("lazy_self", &["main.vn", "helper.vn"]);
    let mut resolver = Resolver::for_script(&dir);
    let info = resolver
        .resolve(&ImportPrefix::Self_, &["helper"], &dir.join("main.vn"))
        .unwrap();
    assert_eq!(info.file_path, dir.join("helper.vn"));
    assert_eq!(info.import_name, "helper");
}

#[test]
fn script_lazy_resolves_parent_import() {
    let dir = setup_script_dir("lazy_parent", &["sub/main.vn", "helper.vn"]);
    let mut resolver = Resolver::for_script(&dir);
    let info = resolver
        .resolve(
            &ImportPrefix::Parent(1),
            &["helper"],
            &dir.join("sub/main.vn"),
        )
        .unwrap();
    assert_eq!(info.file_path, dir.join("helper.vn"));
}

#[test]
fn script_lazy_resolves_self_with_nested_subpath() {
    let dir = setup_script_dir("lazy_nested", &["main.vn", "utils/format.vn"]);
    let mut resolver = Resolver::for_script(&dir);
    let info = resolver
        .resolve(
            &ImportPrefix::Self_,
            &["utils", "format"],
            &dir.join("main.vn"),
        )
        .unwrap();
    assert_eq!(info.file_path, dir.join("utils/format.vn"));
}

#[test]
fn script_import_not_found_errors() {
    let dir = setup_script_dir("not_found", &["main.vn"]);
    let mut resolver = Resolver::for_script(&dir);
    let err = resolver
        .resolve(&ImportPrefix::Self_, &["nonexistent"], &dir.join("main.vn"))
        .unwrap_err();
    assert!(matches!(err, ResolveDiagnostic::NotFound { .. }));
}

#[test]
fn lsp_resolves_vfs_registered_file() {
    let dir = setup_script_dir("vfs_found", &["main.vn"]);
    let vfs_path = dir.join("unwritten.vn");
    let vfs = HashMap::from([(vfs_path.clone(), "".to_string())]);
    let test_fs = TestFileSystem(vfs);
    let mut resolver = Resolver::for_script_with(&dir, Box::new(test_fs));
    let info = resolver
        .resolve(&ImportPrefix::Self_, &["unwritten"], &dir.join("main.vn"))
        .unwrap();
    assert_eq!(info.file_path, vfs_path);
}

#[test]
fn lsp_rejects_unregistered_or_nonexistent_file() {
    let dir = setup_script_dir("vfs_missing", &["main.vn"]);
    let vfs = HashMap::new();
    let test_fs = TestFileSystem(vfs);
    let mut resolver = Resolver::for_script_with(&dir, Box::new(test_fs));
    let err = resolver
        .resolve(&ImportPrefix::Self_, &["ghost"], &dir.join("main.vn"))
        .unwrap_err();
    assert!(matches!(err, ResolveDiagnostic::NotFound { .. }));
}

#[test]
fn empty_manifest_src_has_no_modules() {
    let dir = setup_project("empty_src", &[], &[]);
    fs::create_dir(dir.join("src")).unwrap();
    let resolver = Resolver::for_manifest(&dir).unwrap();
    assert!(resolver.all_modules().is_empty());
}

#[test]
fn non_vn_entry_returns_error_from_detect() {
    let dir = setup_script_dir("bad_entry", &["readme.md"]);
    let entry = dir.join("readme.md");
    let err = Resolver::detect(&entry).unwrap_err();
    assert!(matches!(err, ResolveDiagnostic::NotFound { .. }));
}

#[test]
fn directory_module_dedup() {
    let dir = setup_project("dedup", &["src/foo/foo.vn"], &[]);
    let resolver = Resolver::for_manifest(&dir).unwrap();
    let modules = resolver.all_modules();
    assert_eq!(modules.len(), 1);
    let info = modules.get(&vec!["foo".to_string()]).unwrap();
    assert_eq!(info.import_name, "foo");
    assert_eq!(info.path, vec!["foo"]);
}

#[test]
fn detect_from_directory_entry() {
    let dir = setup_project("dir_entry", &["src/main.vn"], &[]);
    let resolver = Resolver::detect(&dir).unwrap();
    assert_eq!(resolver.mode(), &ResolverMode::Manifest);
    assert_eq!(resolver.root(), &dir);
}

#[test]
fn parent_prefix_at_exact_root_is_valid() {
    let dir = setup_project("parent_root", &["src/foo/child.vn", "src/main.vn"], &[]);
    let mut resolver = Resolver::for_manifest(&dir).unwrap();
    let info = resolver
        .resolve(
            &ImportPrefix::Parent(1),
            &["main"],
            &dir.join("src/foo/child.vn"),
        )
        .unwrap();
    assert_eq!(info.file_path, dir.join("src/main.vn"));
}

#[test]
fn duplicate_resolve_returns_same_module() {
    let dir = setup_project("dup_resolve", &["src/main.vn", "src/helper.vn"], &[]);
    let mut resolver = Resolver::for_manifest(&dir).unwrap();
    let a = resolver
        .resolve(&ImportPrefix::Self_, &["helper"], &dir.join("src/main.vn"))
        .unwrap()
        .file_path;
    let b = resolver
        .resolve(&ImportPrefix::Self_, &["helper"], &dir.join("src/main.vn"))
        .unwrap()
        .file_path;
    assert_eq!(a, b);
}
