//! Lecture et validation de `models/manifest.toml`.

use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;
use std::collections::HashSet;

/// Ensembles connus. Un élément appartient à un ou plusieurs ensembles.
pub const SETS: &[&str] = &["dev", "ci", "optional"];

/// Systèmes pour lesquels un runtime peut être déclaré.
pub const OSES: &[&str] = &["linux", "windows"];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    #[serde(default)]
    pub model: Vec<Model>,
    #[serde(default)]
    pub runtime: Vec<Runtime>,
}

/// Un fichier de modèle, rangé tel quel dans `models/files/`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Model {
    pub id: String,
    pub file: String,
    pub url: String,
    pub size: u64,
    pub sha256: String,
    pub license: String,
    pub sets: Vec<String>,
}

/// Une archive de binaires tiers, extraite dans `runtime/<name>/`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Runtime {
    pub name: String,
    pub os: String,
    pub url: String,
    pub size: u64,
    pub sha256: String,
    pub license: String,
    pub sets: Vec<String>,
}

/// Ce que `fetch` doit récupérer.
#[derive(Debug)]
pub struct Selection<'a> {
    pub models: Vec<&'a Model>,
    pub runtimes: Vec<&'a Runtime>,
}

impl Manifest {
    /// Lit et valide le manifeste.
    pub fn parse(text: &str) -> Result<Self> {
        let manifest: Manifest = toml::from_str(text).context("manifeste illisible")?;
        manifest.validate()?;
        Ok(manifest)
    }

    fn validate(&self) -> Result<()> {
        let mut ids = HashSet::new();
        let mut files = HashSet::new();
        for m in &self.model {
            ensure!(
                ids.insert(m.id.as_str()),
                "identifiant en double : {}",
                m.id
            );
            ensure!(
                files.insert(m.file.as_str()),
                "fichier en double : {}",
                m.file
            );
            ensure!(
                !m.file.is_empty() && !m.file.contains(['/', '\\']) && m.file != "..",
                "{} : `file` doit être un simple nom de fichier",
                m.id
            );
            check_common(&m.id, &m.url, m.size, &m.sha256, &m.license, &m.sets)?;
        }
        let mut runtimes = HashSet::new();
        for r in &self.runtime {
            let label = format!("{} ({})", r.name, r.os);
            ensure!(
                runtimes.insert((r.name.as_str(), r.os.as_str())),
                "runtime en double : {label}"
            );
            ensure!(OSES.contains(&r.os.as_str()), "{label} : système inconnu");
            ensure!(
                !r.name.is_empty() && !r.name.contains(['/', '\\']) && !r.name.starts_with('.'),
                "{label} : `name` doit être un simple nom de dossier"
            );
            ensure!(
                r.url.ends_with(".zip") || r.url.ends_with(".tar.gz"),
                "{label} : seules les archives .zip et .tar.gz sont prises en charge"
            );
            check_common(&label, &r.url, r.size, &r.sha256, &r.license, &r.sets)?;
        }
        Ok(())
    }

    /// Les éléments d'un ensemble pour un système, plus ceux demandés par `with`.
    pub fn select<'a>(&'a self, set: &str, with: &[String], os: &str) -> Result<Selection<'a>> {
        ensure!(
            SETS.contains(&set),
            "ensemble inconnu : {set} (connus : {SETS:?})"
        );
        for id in with {
            if !self.model.iter().any(|m| &m.id == id) {
                bail!("--with {id} : aucun modèle de ce nom dans le manifeste");
            }
        }
        let in_set = |sets: &[String]| sets.iter().any(|s| s == set);
        Ok(Selection {
            models: self
                .model
                .iter()
                .filter(|m| in_set(&m.sets) || with.contains(&m.id))
                .collect(),
            runtimes: self
                .runtime
                .iter()
                .filter(|r| r.os == os && in_set(&r.sets))
                .collect(),
        })
    }
}

fn check_common(
    label: &str,
    url: &str,
    size: u64,
    sha256: &str,
    license: &str,
    sets: &[String],
) -> Result<()> {
    ensure!(
        url.starts_with("https://"),
        "{label} : l'URL doit être en https"
    );
    ensure!(size > 0, "{label} : taille nulle");
    ensure!(
        sha256.len() == 64
            && sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "{label} : SHA-256 invalide (64 caractères hexadécimaux en minuscules)"
    );
    ensure!(!license.is_empty(), "{label} : licence manquante");
    ensure!(!sets.is_empty(), "{label} : aucun ensemble");
    for s in sets {
        ensure!(SETS.contains(&s.as_str()), "{label} : ensemble inconnu {s}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const REAL: &str = include_str!("../../models/manifest.toml");

    const ONE: &str = r#"
        [[model]]
        id = "a"
        file = "a.bin"
        url = "https://example.org/a.bin"
        size = 1
        sha256 = "0000000000000000000000000000000000000000000000000000000000000000"
        license = "MIT"
        sets = ["dev"]
    "#;

    #[test]
    fn le_manifeste_du_depot_est_valide() {
        let m = Manifest::parse(REAL).unwrap();
        assert!(!m.model.is_empty());
        for os in OSES {
            let sel = m.select("dev", &[], os).unwrap();
            assert_eq!(sel.runtimes.len(), 1, "un llama.cpp attendu pour {os}");
        }
    }

    #[test]
    fn l_ensemble_ci_reste_petit() {
        let m = Manifest::parse(REAL).unwrap();
        let sel = m.select("ci", &[], "linux").unwrap();
        let total: u64 = sel.models.iter().map(|m| m.size).sum();
        assert!(total < 400_000_000, "l'ensemble ci pèse {total} octets");
    }

    #[test]
    fn optional_seulement_a_la_demande() {
        let m = Manifest::parse(REAL).unwrap();
        let dev = m.select("dev", &[], "linux").unwrap();
        assert!(dev.models.iter().all(|m| m.id != "ministral-3-14b"));
        let with = m
            .select("dev", &["ministral-3-14b".into()], "linux")
            .unwrap();
        assert!(with.models.iter().any(|m| m.id == "ministral-3-14b"));
    }

    #[test]
    fn with_inconnu_refuse() {
        let m = Manifest::parse(ONE).unwrap();
        assert!(m.select("dev", &["zzz".into()], "linux").is_err());
    }

    #[test]
    fn identifiant_en_double_refuse() {
        let twice = format!("{ONE}{}", ONE.replace("a.bin", "b.bin"));
        assert!(Manifest::parse(&twice).is_err());
    }

    #[test]
    fn sha_invalide_refuse() {
        assert!(Manifest::parse(&ONE.replace("0000000000", "ZZZZZZZZZZ")).is_err());
    }

    #[test]
    fn chemin_dans_file_refuse() {
        assert!(Manifest::parse(&ONE.replace("\"a.bin\"", "\"../a.bin\"")).is_err());
    }

    #[test]
    fn ensemble_inconnu_refuse() {
        assert!(Manifest::parse(&ONE.replace("[\"dev\"]", "[\"prod\"]")).is_err());
    }
}
