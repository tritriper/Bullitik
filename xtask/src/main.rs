//! Tâches du projet Bullitik, lancées par `cargo xtask <commande>`.

mod changelog;
mod fetch;
mod manifest;
mod package;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "cargo xtask", about = "Tâches du projet Bullitik")]
struct Cli {
    /// Dossier de travail (celui qui contient models/manifest.toml). Par défaut :
    /// le dépôt, ou le dossier courant pour un xtask.exe livré hors du dépôt.
    #[arg(long, global = true)]
    root: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Télécharge et vérifie (SHA-256) les modèles et binaires d'un ensemble du manifeste.
    Fetch {
        /// Ensemble à récupérer : dev, ci ou optional.
        #[arg(long, default_value = "dev")]
        set: String,
        /// Ajoute un modèle hors ensemble, par son identifiant (répétable).
        #[arg(long = "with", value_name = "ID")]
        with: Vec<String>,
        /// Affiche ce qui serait téléchargé, sans rien télécharger.
        #[arg(long)]
        dry_run: bool,
    },
    /// Vérifie que models/manifest.toml est valide, sans rien télécharger.
    CheckManifest,
    /// Assemble le dossier portable des spikes (après `cargo build --release --workspace --bins`).
    Package {
        /// Dossier de sortie.
        #[arg(long, default_value = "dist")]
        out: PathBuf,
        /// Version (par défaut : BULLITIK_VERSION, sinon « dev »).
        #[arg(long)]
        version: Option<String>,
        /// Produire aussi le zip.
        #[arg(long)]
        zip: bool,
    },
    /// Vérifie ou lit CHANGELOG.md.
    Changelog {
        #[command(subcommand)]
        command: ChangelogCommand,
    },
}

#[derive(Subcommand)]
enum ChangelogCommand {
    /// Vérifie le format du CHANGELOG (règles d'AGENTS.md).
    Check,
    /// Affiche le corps d'une version, pour le texte d'une release.
    Extract { version: String },
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("erreur : {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<()> {
    let root = cli.root.clone().unwrap_or_else(default_root);
    match cli.command {
        Command::Fetch { set, with, dry_run } => {
            let manifest = load_manifest(&root)?;
            fetch::run(
                &root,
                &manifest,
                &fetch::Options {
                    set: &set,
                    with: &with,
                    dry_run,
                },
            )
        }
        Command::CheckManifest => {
            let m = load_manifest(&root)?;
            println!(
                "manifeste valide : {} modèle(s), {} runtime(s)",
                m.model.len(),
                m.runtime.len()
            );
            Ok(())
        }
        Command::Package { out, version, zip } => {
            let version = version
                .or_else(|| std::env::var("BULLITIK_VERSION").ok())
                .unwrap_or_else(|| "dev".into());
            package::run(
                &root,
                &package::Options {
                    out: &out,
                    version: &version,
                    zip,
                },
            )?;
            Ok(())
        }
        Command::Changelog { command } => {
            let text = std::fs::read_to_string(root.join("CHANGELOG.md"))
                .context("lecture de CHANGELOG.md")?;
            match command {
                ChangelogCommand::Check => {
                    let errors = changelog::check(&text);
                    if errors.is_empty() {
                        println!("CHANGELOG.md conforme");
                        return Ok(());
                    }
                    for e in &errors {
                        eprintln!("CHANGELOG.md, {e}");
                    }
                    bail!("{} écart(s) dans CHANGELOG.md", errors.len())
                }
                ChangelogCommand::Extract { version } => {
                    let version = version.strip_prefix('v').unwrap_or(&version);
                    match changelog::extract(&text, version) {
                        Some(body) => {
                            println!("{body}");
                            Ok(())
                        }
                        None => bail!("aucune section [{version}] dans CHANGELOG.md"),
                    }
                }
            }
        }
    }
}

fn load_manifest(root: &Path) -> Result<manifest::Manifest> {
    let path = root.join("models").join("manifest.toml");
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("lecture de {}", path.display()))?;
    manifest::Manifest::parse(&text).with_context(|| path.display().to_string())
}

/// Le dossier de travail, dans cet ordre :
///   1. le dossier courant s'il contient `models/manifest.toml` (cas de
///      `xtask.exe` lancé dans le dossier livré des spikes, et de `cargo xtask`
///      lancé à la racine du dépôt) ;
///   2. la racine du dépôt où xtask a été compilé, si elle existe encore ;
///   3. le dossier courant.
fn default_root() -> PathBuf {
    let has_manifest = |dir: &Path| dir.join("models").join("manifest.toml").is_file();
    let cwd = PathBuf::from(".");
    if has_manifest(&cwd) {
        return cwd;
    }
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();
    if has_manifest(&repo) { repo } else { cwd }
}
