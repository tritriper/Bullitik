//! `cargo xtask package` : assemble le dossier portable des spikes, et son zip.
//!
//! Le même code sert la CI (artefact de chaque push) et la CD (release de
//! chaque tag) : ce qui est testé est ce qui est livré.

use anyhow::{Context, Result, bail, ensure};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Les exécutables livrés, sans extension.
const BINARIES: &[&str] = &[
    "spike-stt",
    "spike-tts",
    "spike-llm",
    "spike-chaine",
    "xtask",
];

pub struct Options<'a> {
    /// Dossier où créer le dossier livré (et le zip).
    pub out: &'a Path,
    /// Version affichée dans le nom du zip et le fichier VERSION.
    pub version: &'a str,
    /// Produire aussi le zip.
    pub zip: bool,
}

/// Assemble `<out>/bullitik-spikes-<os>/` et, si demandé, le zip à côté.
/// Renvoie le chemin du dossier.
pub fn run(root: &Path, opts: &Options) -> Result<PathBuf> {
    let os = std::env::consts::OS;
    let target = target_dir(root).join("release");
    let name = format!("bullitik-spikes-{os}");
    let dir = opts.out.join(&name);
    if dir.exists() {
        fs::remove_dir_all(&dir).with_context(|| format!("nettoyage de {}", dir.display()))?;
    }
    fs::create_dir_all(dir.join("models"))?;

    for bin in BINARIES {
        let file = format!("{bin}{}", std::env::consts::EXE_SUFFIX);
        let src = target.join(&file);
        ensure!(
            src.is_file(),
            "{} absent : lancer `cargo build --release --workspace --bins`",
            src.display()
        );
        fs::copy(&src, dir.join(&file))?;
    }
    fs::copy(
        root.join("models/manifest.toml"),
        dir.join("models/manifest.toml"),
    )?;
    copy_dir(&root.join("content"), &dir.join("content"))?;
    copy_dir(&espeak_data(&target)?, &dir.join("espeak-ng-data"))?;
    fs::create_dir_all(dir.join("testdata"))?;
    fs::copy(
        root.join("crates/voice/testdata/phrase.wav"),
        dir.join("testdata/phrase.wav"),
    )?;
    fs::copy(
        root.join("crates/voice/testdata/README.md"),
        dir.join("testdata/README.md"),
    )?;
    fs::copy(root.join("LICENSE"), dir.join("LICENSE.txt"))?;
    fs::copy(
        root.join("docs/licences-tierces.md"),
        dir.join("LICENCES-TIERCES.md"),
    )?;
    let readme = if os == "windows" {
        "docs/spikes/windows.md"
    } else {
        "docs/spikes/linux.md"
    };
    fs::copy(root.join(readme), dir.join("LISEZMOI.md"))?;
    if os == "windows" {
        fs::copy(
            root.join("scripts/mesurer-spikes.ps1"),
            dir.join("mesurer.ps1"),
        )?;
    }
    fs::write(dir.join("VERSION"), format!("{}\n", opts.version))?;
    eprintln!("dossier prêt : {}", dir.display());

    if opts.zip {
        let zip_path = opts
            .out
            .join(format!("Bullitik-spikes-{}-{os}-x64.zip", opts.version));
        zip_dir(&dir, &name, &zip_path)?;
        eprintln!("zip prêt : {}", zip_path.display());
    }
    Ok(dir)
}

fn target_dir(root: &Path) -> PathBuf {
    std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from)
}

/// Les données d'espeak-ng générées au build par espeak-rs-sys (la plus récente
/// si plusieurs builds coexistent dans le cache).
fn espeak_data(target: &Path) -> Result<PathBuf> {
    let build = target.join("build");
    let found = fs::read_dir(&build)
        .with_context(|| format!("lecture de {}", build.display()))?
        .flatten()
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("espeak-rs-sys-")
        })
        .map(|e| e.path().join("out").join("share").join("espeak-ng-data"))
        .filter(|p| p.is_dir())
        .max_by_key(|p| p.metadata().and_then(|m| m.modified()).ok());
    match found {
        Some(p) => Ok(p),
        None => bail!("espeak-ng-data introuvable sous {}", build.display()),
    }
}

fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src).with_context(|| format!("lecture de {}", src.display()))? {
        let entry = entry?;
        let to = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &to)?;
        } else {
            fs::copy(entry.path(), &to)?;
        }
    }
    Ok(())
}

/// Zippe `dir` sous le nom `prefix/` ; les exécutables gardent le droit
/// d'exécution (Linux).
fn zip_dir(dir: &Path, prefix: &str, zip_path: &Path) -> Result<()> {
    let mut zip = zip::ZipWriter::new(File::create(zip_path)?);
    let mut files = Vec::new();
    collect(dir, &mut files)?;
    files.sort();
    for path in files {
        let rel = path.strip_prefix(dir)?.to_string_lossy().replace('\\', "/");
        let executable = BINARIES.iter().any(|b| rel == *b);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(if executable { 0o755 } else { 0o644 });
        zip.start_file(format!("{prefix}/{rel}"), options)?;
        io::copy(&mut File::open(&path)?, &mut zip)?;
    }
    zip.finish()?.flush()?;
    Ok(())
}

fn collect(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            collect(&entry.path(), files)?;
        } else {
            files.push(entry.path());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zip_garde_l_arborescence_et_les_droits() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("d");
        fs::create_dir_all(dir.join("sous")).unwrap();
        fs::write(dir.join("xtask"), b"bin").unwrap();
        fs::write(dir.join("sous/a.txt"), b"a").unwrap();
        let zip_path = tmp.path().join("x.zip");
        zip_dir(&dir, "racine", &zip_path).unwrap();
        let mut archive = zip::ZipArchive::new(File::open(&zip_path).unwrap()).unwrap();
        let names: Vec<String> = archive
            .file_names()
            .map(|n| n.unwrap().into_owned())
            .collect();
        assert!(names.contains(&"racine/xtask".to_string()), "{names:?}");
        assert!(
            names.contains(&"racine/sous/a.txt".to_string()),
            "{names:?}"
        );
        // Sous Windows, la crate zip écrit des entrées « DOS » sans droits Unix ;
        // ils ne comptent que pour le zip Linux, construit sous Linux.
        if cfg!(unix) {
            assert_eq!(
                archive
                    .by_name("racine/xtask")
                    .unwrap()
                    .unix_mode()
                    .map(|m| m & 0o777),
                Some(0o755)
            );
        }
    }

    #[test]
    fn copie_recursive() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("a/b")).unwrap();
        fs::write(tmp.path().join("a/b/c"), b"x").unwrap();
        copy_dir(&tmp.path().join("a"), &tmp.path().join("z")).unwrap();
        assert_eq!(fs::read(tmp.path().join("z/b/c")).unwrap(), b"x");
    }
}
