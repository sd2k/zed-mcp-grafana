use std::{env, fs};

use serde::Deserialize;
use zed_extension_api::{
    self as zed, Command, ContextServerId, Project, Result, serde_json,
    settings::ContextServerSettings,
};

const REPO_NAME: &str = "grafana/mcp-grafana";
const BINARY_NAME: &str = "mcp-grafana";

#[derive(Debug, Deserialize)]
struct GrafanaContextServerSettings {
    /// The URL of the Grafana instance.
    ///
    /// Note this is marked as optional because it may come from the
    /// `GRAFANA_URL` environment variable instead.
    #[serde(default)]
    grafana_url: Option<String>,

    /// The API key of the Grafana instance.
    ///
    /// This is optional if the Grafana instance is accessible without
    /// authentication. It can also be set using the `GRAFANA_API_KEY`
    /// environment variable.
    #[serde(default)]
    grafana_api_key: Option<String>,

    /// Enabled categories of tools.
    ///
    /// See the [README of the Grafana MCP server][readme] to see the list
    /// of available categories.
    ///
    /// Defaults to `None`, which means all tools are enabled.
    ///
    /// [readme]: https://github.com/grafana/mcp-grafana
    #[serde(default)]
    enabled_tools: Option<Vec<String>>,

    /// Enable the Grafana MCP server's debug flag.
    ///
    /// This will cause requests to and responses from the Grafana
    /// instance to be logged by the MCP server.
    ///
    /// Defaults to false.
    #[serde(default)]
    debug: bool,
}

struct GrafanaModelContextExtension {
    cached_binary_path: Option<String>,
}

impl GrafanaModelContextExtension {
    fn context_server_binary_path(
        &mut self,
        _context_server_id: &ContextServerId,
    ) -> Result<String> {
        if let Some(path) = &self.cached_binary_path {
            if fs::metadata(path).is_ok_and(|stat| stat.is_file()) {
                return Ok(path.clone());
            }
        }

        let release = zed::latest_github_release(
            REPO_NAME,
            zed::GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;

        let (platform, arch) = zed::current_platform();
        let asset_name = format!(
            "{BINARY_NAME}_{os}_{arch}.{ext}",
            arch = match arch {
                zed::Architecture::Aarch64 => "arm64",
                zed::Architecture::X86 => "i386",
                zed::Architecture::X8664 => "x86_64",
            },
            os = match platform {
                zed::Os::Mac => "Darwin",
                zed::Os::Linux => "Linux",
                zed::Os::Windows => "Windows",
            },
            ext = match platform {
                zed::Os::Mac | zed::Os::Linux => "tar.gz",
                zed::Os::Windows => "zip",
            }
        );

        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == asset_name)
            .ok_or_else(|| format!("no asset found matching {:?}", asset_name))?;

        let version_dir = format!("{BINARY_NAME}-{}", release.version);
        fs::create_dir_all(&version_dir)
            .map_err(|err| format!("failed to create directory '{version_dir}': {err}"))?;
        let binary_path = format!("{version_dir}/{BINARY_NAME}");

        if !fs::metadata(&binary_path).is_ok_and(|stat| stat.is_file()) {
            let file_kind = match platform {
                zed::Os::Mac | zed::Os::Linux => zed::DownloadedFileType::GzipTar,
                zed::Os::Windows => zed::DownloadedFileType::Zip,
            };

            zed::download_file(&asset.download_url, &version_dir, file_kind)
                .map_err(|e| format!("failed to download file: {e}"))?;

            zed::make_file_executable(&binary_path)?;

            // Removes old versions
            let entries =
                fs::read_dir(".").map_err(|e| format!("failed to list working directory {e}"))?;
            for entry in entries {
                let entry = entry.map_err(|e| format!("failed to load directory entry {e}"))?;
                if entry.file_name().to_str() != Some(&version_dir) {
                    fs::remove_dir_all(entry.path()).ok();
                }
            }
        }

        self.cached_binary_path = Some(binary_path.clone());
        Ok(binary_path)
    }
}

impl zed::Extension for GrafanaModelContextExtension {
    fn new() -> Self {
        Self {
            cached_binary_path: None,
        }
    }

    fn context_server_command(
        &mut self,
        context_server_id: &ContextServerId,
        project: &Project,
    ) -> Result<Command> {
        let settings = ContextServerSettings::for_project("mcp-grafana", project)?;
        let Some(settings) = settings.settings else {
            return Err("missing Grafana settings".into());
        };
        let settings: GrafanaContextServerSettings =
            serde_json::from_value(settings).map_err(|e| e.to_string())?;

        let Some(grafana_url) = env::var("GRAFANA_URL").ok().or(settings.grafana_url) else {
            return Err(
                "missing Grafana URL; configure in `grafana_url` setting or GRAFANA_URL env var"
                    .into(),
            );
        };
        let api_key = env::var("GRAFANA_API_KEY")
            .ok()
            .or(settings.grafana_api_key);

        let mut env = vec![("GRAFANA_URL".into(), grafana_url)];
        if let Some(api_key) = api_key {
            env.push(("GRAFANA_API_KEY".into(), api_key));
        }

        let mut args = vec![];
        if let Some(enabled_tools) = settings.enabled_tools {
            args.push("--enabled-tools".into());
            args.push(enabled_tools.join(","));
        }
        if settings.debug {
            args.push("--debug".into());
        }

        Ok(Command {
            command: self.context_server_binary_path(context_server_id)?,
            args,
            env,
        })
    }
}

zed::register_extension!(GrafanaModelContextExtension);
