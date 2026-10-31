//! `cargo xtask fetch` : téléchargement vérifié des modèles et des runtimes.
//!
//! Chaque fichier est écrit dans un `.part`, haché pendant le téléchargement,
//! puis renommé seulement si sa taille et son SHA-256 sont ceux du manifeste.
//! Un fichier déjà présent et intègre n'est pas retéléchargé.

use crate::manifest::{Manifest, Model, Runtime};
use anyhow::{Context, Result, bail, ensure};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{self, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Fichier qui, dans `runtime/<name>/`, mémorise le SHA-256 de l'archive extraite.
const RUNTIME_STAMP: &str = ".archive-sha256";

pub struct Options<'a> {
    pub set: &'a str,
    pub with: &'a [String],
    pub dry_run: bool,
}

pub fn run(root: &Path, manifest: &Manifest, opts: &Options) -> Result<()> {
    let sel = manifest.select(opts.set, opts.with, std::env::consts::OS)?;
    let total: u64 = sel
        .models
        .iter()
        .map(|m| m.size)
        .chain(sel.runtimes.iter().map(|r| r.size))
        .sum();
    eprintln!(
        "ensemble « {} » : {} modèle(s), {} runtime(s), {} au total",
        opts.set,
        sel.models.len(),
        sel.runtimes.len(),
        human(total)
    );
    if opts.dry_run {
        for m in &sel.models {
            eprintln!("  {:<26} {:>9}  {}", m.id, human(m.size), m.license);
        }
        for r in &sel.runtimes {
            eprintln!("  {:<26} {:>9}  {}", r.name, human(r.size), r.license);
        }
        return Ok(());
    }
    let agent = agent();
    let models_dir = root.join("models").join("files");
    fs::create_dir_all(&models_dir)?;
    for m in &sel.models {
        fetch_model(&agent, &models_dir, m).with_context(|| format!("modèle {}", m.id))?;
    }
    for r in &sel.runtimes {
        fetch_runtime(&agent, &root.join("runtime"), r)
            .with_context(|| format!("runtime {}", r.name))?;
    }
    eprintln!("tout est en place.");
    Ok(())
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .tls_config(
            ureq::tls::TlsConfig::builder()
                // Magasin de certificats du système : marche derrière un proxy d'entreprise.
                .root_certs(ureq::tls::RootCerts::PlatformVerifier)
                .build(),
        )
        .timeout_connect(Some(Duration::from_secs(30)))
        .build()
        .into()
}

fn fetch_model(agent: &ureq::Agent, dir: &Path, m: &Model) -> Result<()> {
    let dest = dir.join(&m.file);
    if dest.exists() {
        if file_matches(&dest, m.size, &m.sha256)? {
            eprintln!("✔ {} déjà présent", m.file);
            return Ok(());
        }
        eprintln!(
            "✘ {} présent mais différent du manifeste : retéléchargement",
            m.file
        );
    }
    download(agent, &m.url, m.size, &m.sha256, &dest)
}

fn fetch_runtime(agent: &ureq::Agent, dir: &Path, r: &Runtime) -> Result<()> {
    let target = dir.join(&r.name);
    let stamp = target.join(RUNTIME_STAMP);
    if fs::read_to_string(&stamp).is_ok_and(|s| s.trim() == r.sha256) {
        eprintln!("✔ {} déjà extrait", r.name);
        return Ok(());
    }
    let downloads = dir.join(".downloads");
    fs::create_dir_all(&downloads)?;
    let archive_name = r
        .url
        .rsplit('/')
        .next()
        .context("URL sans nom de fichier")?;
    let archive = downloads.join(archive_name);
    if !(archive.exists() && file_matches(&archive, r.size, &r.sha256)?) {
        download(agent, &r.url, r.size, &r.sha256, &archive)?;
    }
    let tmp = dir.join(format!(".{}.tmp", r.name));
    if tmp.exists() {
        fs::remove_dir_all(&tmp)?;
    }
    fs::create_dir_all(&tmp)?;
    eprintln!("  extraction de {archive_name}");
    if archive_name.ends_with(".zip") {
        zip::ZipArchive::new(File::open(&archive)?)?.extract(&tmp)?;
    } else {
        tar::Archive::new(flate2::read::GzDecoder::new(File::open(&archive)?)).unpack(&tmp)?;
    }
    let content = single_top_dir(&tmp)?.unwrap_or_else(|| tmp.clone());
    if target.exists() {
        fs::remove_dir_all(&target)?;
    }
    fs::rename(&content, &target)?;
    if tmp.exists() {
        fs::remove_dir_all(&tmp)?;
    }
    fs::write(&stamp, &r.sha256)?;
    eprintln!("✔ {} extrait dans {}", r.name, target.display());
    Ok(())
}

/// Si `dir` ne contient qu'un dossier (archive avec un dossier racine), le renvoie.
fn single_top_dir(dir: &Path) -> Result<Option<PathBuf>> {
    let entries: Vec<_> = fs::read_dir(dir)?.collect::<io::Result<_>>()?;
    Ok(match entries.as_slice() {
        [only] if only.file_type()?.is_dir() => Some(only.path()),
        _ => None,
    })
}

fn download(agent: &ureq::Agent, url: &str, size: u64, sha256: &str, dest: &Path) -> Result<()> {
    let name = dest.file_name().unwrap_or_default().to_string_lossy();
    eprintln!("↓ {name} ({})", human(size));
    let part = dest.with_extension(format!(
        "{}part",
        dest.extension()
            .map(|e| format!("{}.", e.to_string_lossy()))
            .unwrap_or_default()
    ));
    let mut resp = agent
        .get(url)
        .call()
        .with_context(|| format!("GET {url}"))?;
    let mut reader = resp.body_mut().as_reader();
    let mut out = io::BufWriter::new(File::create(&part)?);
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    let (mut done, mut shown) = (0u64, 0u64);
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        out.write_all(&buf[..n])?;
        done += n as u64;
        ensure!(
            done <= size,
            "le serveur envoie plus que les {size} octets attendus"
        );
        let pct = done * 100 / size;
        if pct >= shown + 5 {
            shown = pct - pct % 5;
            eprint!("\r  {shown:>3} %");
        }
    }
    out.flush()?;
    drop(out);
    eprintln!();
    let got = hex(&hasher.finalize());
    if done != size || got != sha256 {
        let _ = fs::remove_file(&part);
        bail!(
            "{name} : reçu {done} octets de SHA-256 {got}, attendu {size} octets de SHA-256 {sha256}"
        );
    }
    fs::rename(&part, dest)?;
    fs::write(stamp_path(dest), stamp_line(&fs::metadata(dest)?, sha256))?;
    eprintln!("✔ {name} vérifié");
    Ok(())
}

/// Vrai si le fichier a la taille et le SHA-256 attendus. Le hachage d'un fichier
/// déjà vérifié est mémorisé à côté de lui (`<fichier>.verified`) et n'est refait
/// que si sa taille ou sa date de modification change.
fn file_matches(path: &Path, size: u64, sha256: &str) -> Result<bool> {
    let meta = fs::metadata(path)?;
    if meta.len() != size {
        return Ok(false);
    }
    let stamp = stamp_path(path);
    let line = stamp_line(&meta, sha256);
    if fs::read_to_string(&stamp).is_ok_and(|s| s == line) {
        return Ok(true);
    }
    let ok = sha256_file(path)? == sha256;
    if ok {
        fs::write(&stamp, line)?;
    }
    Ok(ok)
}

fn stamp_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".verified");
    PathBuf::from(name)
}

fn stamp_line(meta: &fs::Metadata, sha256: &str) -> String {
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_nanos());
    format!("{sha256} {} {mtime}", meta.len())
}

fn sha256_file(path: &Path) -> Result<String> {
    let mut reader = BufReader::with_capacity(1 << 20, File::open(path)?);
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        match reader.read(&mut buf)? {
            0 => break,
            n => hasher.update(&buf[..n]),
        }
    }
    Ok(hex(&hasher.finalize()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn human(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["o", "Ko", "Mo", "Go"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1000.0 && unit < UNITS.len() - 1 {
        value /= 1000.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} o")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_connu() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("x");
        fs::write(&f, b"abc").unwrap();
        assert_eq!(
            sha256_file(&f).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert!(
            file_matches(
                &f,
                3,
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
            )
            .unwrap()
        );
        assert!(
            !file_matches(
                &f,
                4,
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
            )
            .unwrap()
        );
    }

    #[test]
    fn fichier_modifie_rehache_malgre_le_cache() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("x");
        let abc = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        fs::write(&f, b"abc").unwrap();
        assert!(file_matches(&f, 3, abc).unwrap());
        assert!(stamp_path(&f).exists());
        fs::write(&f, b"abd").unwrap();
        let later = std::time::SystemTime::now() + Duration::from_secs(5);
        File::options()
            .write(true)
            .open(&f)
            .unwrap()
            .set_modified(later)
            .unwrap();
        assert!(!file_matches(&f, 3, abc).unwrap());
    }

    #[test]
    fn dossier_racine_unique_detecte() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("llama-b1")).unwrap();
        assert_eq!(
            single_top_dir(dir.path()).unwrap(),
            Some(dir.path().join("llama-b1"))
        );
        fs::write(dir.path().join("autre"), b"").unwrap();
        assert_eq!(single_top_dir(dir.path()).unwrap(), None);
    }

    #[test]
    fn tailles_lisibles() {
        assert_eq!(human(999), "999 o");
        assert_eq!(human(5_198_911_904), "5.2 Go");
    }
}
