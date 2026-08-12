use zed_extension_api::{
    self as zed, Architecture, DownloadedFileType, GithubReleaseOptions, Os, Result,
};

const SERVER_NAME: &str = "vinyl-lsp";
const GITHUB_REPO: &str = "MordechaiHadad/vinyl-lang";

struct VinylExtension {
    cached_binary_path: Option<String>,
}

impl zed::Extension for VinylExtension {
    fn new() -> Self {
        Self {
            cached_binary_path: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let env = worktree.shell_env();

        if let Some(path) = self.cached_binary_path.clone() {
            return Ok(zed::Command {
                command: path,
                args: Vec::new(),
                env,
            });
        }

        if let Some(path) = worktree.which(SERVER_NAME) {
            self.cached_binary_path = Some(path.clone());
            return Ok(zed::Command {
                command: path,
                args: Vec::new(),
                env,
            });
        }

        let latest_release = zed::latest_github_release(
            GITHUB_REPO,
            GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::Downloading,
        );

        let (os, arch) = zed::current_platform();
        let (triple, file_type) = match (os, arch) {
            (Os::Mac, Architecture::Aarch64) => {
                ("aarch64-apple-darwin", DownloadedFileType::GzipTar)
            }
            (Os::Mac, Architecture::X8664) => {
                ("x86_64-apple-darwin", DownloadedFileType::GzipTar)
            }
            (Os::Linux, Architecture::Aarch64) => {
                ("aarch64-unknown-linux-gnu", DownloadedFileType::GzipTar)
            }
            (Os::Linux, Architecture::X8664) => {
                ("x86_64-unknown-linux-gnu", DownloadedFileType::GzipTar)
            }
            (Os::Windows, Architecture::Aarch64) => {
                ("aarch64-pc-windows-msvc", DownloadedFileType::Zip)
            }
            (Os::Windows, Architecture::X8664) => {
                ("x86_64-pc-windows-msvc", DownloadedFileType::Zip)
            }
            _ => return Err(format!("unsupported platform: {os:?} {arch:?}")),
        };

        let asset_name = format!(
            "vinyl-lsp-{triple}.{}",
            if matches!(file_type, DownloadedFileType::Zip) {
                "zip"
            } else {
                "tar.gz"
            }
        );
        let asset = latest_release
            .assets
            .iter()
            .find(|asset| asset.name == asset_name)
            .ok_or_else(|| {
                format!("could not find asset {asset_name} in latest {GITHUB_REPO} release")
            })?;

        let version_dir = format!("{SERVER_NAME}-{}", latest_release.version);
        let binary_name = if os == Os::Windows {
            "vinyl-lsp.exe"
        } else {
            "vinyl-lsp"
        };
        let binary_path = format!("{version_dir}/{binary_name}");

        zed::download_file(&asset.download_url, &version_dir, file_type)?;
        zed::make_file_executable(&binary_path)?;

        self.cached_binary_path = Some(binary_path.clone());

        Ok(zed::Command {
            command: binary_path,
            args: Vec::new(),
            env,
        })
    }
}

zed::register_extension!(VinylExtension);