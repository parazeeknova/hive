mod config;
mod guard;
mod mcp;
mod platform;
mod rpc;
#[cfg(all(test, not(miri)))]
mod tests;
mod worker;

use anyhow::{Context, Result, bail, ensure};
use config::{Approval, Config};
use std::{collections::BTreeMap, path::PathBuf};

const HELP: &str = "Hive Local — background bridge for your hosted Hive MCP endpoint

  hive-local init --hive https://HOST/mcp --key-file PATH --root FOLDER
  hive-local enable filesystem|markitdown|duckdb|ffmpeg|serena [--project FOLDER] [--database FILE]
  hive-local disable SLUG
  hive-local list
  hive-local run

All commands accept --config PATH (an absolute config file). Only enable integrations you
want to run. Package versions and tool lists refresh automatically while running.
The agent connects only to your hosted /mcp URL; this program opens no local port.";

#[tokio::main]
#[allow(clippy::too_many_lines)]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{HELP}");
        return Ok(());
    }
    let flag = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let file = flag("--config").map_or_else(platform::config_path, |p| Ok(PathBuf::from(p)))?;
    match args[0].as_str() {
        "init" => {
            ensure!(
                !file.exists(),
                "configuration already exists; edit it to change approved folders"
            );
            let hive = flag("--hive").context("--hive is required")?;
            config::validate_url(&hive)?;
            let key_file = PathBuf::from(flag("--key-file").context("--key-file is required")?)
                .canonicalize()?;
            let roots = args
                .iter()
                .enumerate()
                .filter(|(_, v)| v.as_str() == "--root")
                .map(|(i, _)| {
                    args.get(i + 1)
                        .context("--root needs a directory")
                        .and_then(|p| Ok(PathBuf::from(p).canonicalize()?))
                })
                .collect::<Result<Vec<_>>>()?;
            let shared = args.iter().any(|a| a == "--shared");
            ensure!(
                shared || !roots.is_empty(),
                "at least one approved --root is required"
            );
            let config = Config {
                hive,
                key_file,
                roots,
                enabled: BTreeMap::new(),
                shared,
            };
            // Validate before writing to avoid leaving an unusable config.
            for root in &config.roots {
                ensure!(
                    root.is_dir() && root.parent().is_some() && !config.key_file.starts_with(root),
                    "roots must be directories and cannot contain the Hive key"
                );
            }
            config.write(&file)?;
            println!(
                "Saved {}. Enable integrations, then run hive-local run.",
                file.display()
            );
        }
        "enable" => {
            let slug = args
                .get(1)
                .filter(|s| !s.starts_with('-'))
                .context("integration slug is required")?;
            let mut config = Config::read(&file)?;
            let catalog = rpc::Hive::new(&config)?.catalog().await?;
            let integration = catalog
                .integrations
                .iter()
                .find(|i| &i.slug == slug)
                .context("unknown integration")?;
            let launch = if config.shared {
                integration.shared_launch.as_ref()
            } else {
                integration.local.as_ref()
            }
            .context("integration is not available in this mode")?;
            let project = flag("--project")
                .map(PathBuf::from)
                .map(|p| p.canonicalize())
                .transpose()?;
            ensure!(
                slug != "serena" || project.is_some(),
                "Serena needs --project inside an approved folder"
            );
            let approval = Approval {
                command: launch.command.clone(),
                package: launch.package.clone(),
                project,
                database: flag("--database").map(PathBuf::from),
            };
            launch.validate(&approval)?;
            worker::values(&config, &approval, slug)?;
            config.enabled.insert(slug.clone(), approval);
            config.write(&file)?;
            println!("Enabled {slug}. The running helper picks this up automatically.");
        }
        "disable" => {
            let slug = args.get(1).context("integration slug is required")?;
            let mut config = Config::read(&file)?;
            config.enabled.remove(slug);
            config.write(&file)?;
            println!("Disabled {slug}. An active call may finish before the helper disconnects.");
        }
        "list" => {
            let config = Config::read(&file)?;
            let catalog = rpc::Hive::new(&config)?.catalog().await?;
            for item in catalog.integrations.iter().filter(|i| {
                if config.shared {
                    i.shared_launch.is_some()
                } else {
                    i.local.is_some()
                }
            }) {
                println!(
                    "{}\t{}",
                    item.slug,
                    if config.enabled.contains_key(&item.slug) {
                        "enabled"
                    } else {
                        "disabled"
                    }
                );
            }
        }
        "run" => worker::run(&file).await?,
        _ => bail!("unknown command\n{HELP}"),
    }
    Ok(())
}
