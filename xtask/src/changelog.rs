//! Lecture et vérification de `CHANGELOG.md` (voir AGENTS.md).

/// Sous-sections autorisées dans une version.
pub const SUBSECTIONS: &[&str] = &[
    "Ajouté",
    "Modifié",
    "Corrigé",
    "Limitations connues",
    "CI / build",
];

const UNRELEASED: &str = "## [Non publié]";

/// Corps de la section `## [<version>]`, sans les lignes vides des bords.
/// `None` si la section n'existe pas.
pub fn extract(text: &str, version: &str) -> Option<String> {
    let wanted = format!("## [{version}]");
    let mut lines = text.lines();
    lines.find(|l| *l == wanted || l.starts_with(&format!("{wanted} ")))?;
    let body: Vec<&str> = lines.take_while(|l| !l.starts_with("## ")).collect();
    let start = body
        .iter()
        .position(|l| !l.trim().is_empty())
        .unwrap_or(body.len());
    let end = body
        .iter()
        .rposition(|l| !l.trim().is_empty())
        .map_or(start, |i| i + 1);
    Some(body[start..end].join("\n"))
}

/// Liste les écarts aux règles du CHANGELOG. Vide si tout va bien.
pub fn check(text: &str) -> Vec<String> {
    let mut errors = Vec::new();
    let mut lines = text.lines().enumerate().peekable();
    if lines.peek().map(|(_, l)| *l) != Some("# Changelog") {
        errors.push("ligne 1 : le fichier doit commencer par `# Changelog`".into());
    }
    let mut seen_section = false;
    let mut versions = Vec::<String>::new();
    let mut in_bullet = false;
    for (i, line) in lines {
        let n = i + 1;
        if let Some(rest) = line.strip_prefix("## ") {
            in_bullet = false;
            if line == UNRELEASED {
                if seen_section {
                    errors.push(format!(
                        "ligne {n} : `{UNRELEASED}` doit être la première section"
                    ));
                }
            } else if let Some(v) = parse_release_heading(rest) {
                if versions.contains(&v) {
                    errors.push(format!("ligne {n} : version {v} en double"));
                }
                versions.push(v);
            } else {
                errors.push(format!(
                    "ligne {n} : titre de version attendu, `{UNRELEASED}` ou `## [x.y.z] — AAAA-MM-JJ`"
                ));
            }
            seen_section = true;
        } else if let Some(name) = line.strip_prefix("### ") {
            in_bullet = false;
            if !seen_section {
                errors.push(format!("ligne {n} : sous-section hors d'une version"));
            } else if !SUBSECTIONS.contains(&name) {
                errors.push(format!(
                    "ligne {n} : sous-section inconnue « {name} » (permises : {SUBSECTIONS:?})"
                ));
            }
        } else if !seen_section || is_link_reference(line) {
            // Préambule libre ; liens de comparaison en bas du fichier.
        } else if line.starts_with("- ") {
            in_bullet = true;
        } else if line.trim().is_empty() {
            in_bullet = false;
        } else if in_bullet {
            errors.push(format!("ligne {n} : une entrée tient sur une seule ligne"));
        } else {
            errors.push(format!(
                "ligne {n} : seules des entrées `- …` sont permises dans une version"
            ));
        }
    }
    errors
}

/// `[1.2.3] — 2026-10-31` → `Some("1.2.3")`.
fn parse_release_heading(rest: &str) -> Option<String> {
    let rest = rest.strip_prefix('[')?;
    let (version, date) = rest.split_once("] — ")?;
    let semver = version.split('.').collect::<Vec<_>>();
    let numeric = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let date_ok = date.len() == 10
        && date.bytes().enumerate().all(|(i, b)| {
            if i == 4 || i == 7 {
                b == b'-'
            } else {
                b.is_ascii_digit()
            }
        });
    (semver.len() == 3 && semver.iter().all(|s| numeric(s)) && date_ok).then(|| version.to_string())
}

fn is_link_reference(line: &str) -> bool {
    line.starts_with('[') && line.contains("]: https://")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "# Changelog\n\nPréambule libre.\n\n## [Non publié]\n\n### Ajouté\n- Une chose.\n\n## [0.1.0] — 2026-11-02\n\n### Corrigé\n- Un bug.\n- Un autre.\n\n[0.1.0]: https://github.com/tritriper/Bullitik/compare/v0.0.0...v0.1.0\n";

    #[test]
    fn le_changelog_du_depot_est_conforme() {
        let errors = check(include_str!("../../CHANGELOG.md"));
        assert!(errors.is_empty(), "{errors:#?}");
    }

    #[test]
    fn exemple_conforme() {
        assert_eq!(check(SAMPLE), Vec::<String>::new());
    }

    #[test]
    fn extrait_une_version() {
        assert_eq!(
            extract(SAMPLE, "0.1.0").unwrap(),
            "### Corrigé\n- Un bug.\n- Un autre.\n\n[0.1.0]: https://github.com/tritriper/Bullitik/compare/v0.0.0...v0.1.0"
        );
        assert_eq!(
            extract(SAMPLE, "Non publié").unwrap(),
            "### Ajouté\n- Une chose."
        );
        assert!(extract(SAMPLE, "9.9.9").is_none());
    }

    #[test]
    fn entree_sur_deux_lignes_refusee() {
        let bad = SAMPLE.replace("- Un bug.\n", "- Un bug\n  qui continue.\n");
        assert_eq!(check(&bad).len(), 1);
    }

    #[test]
    fn sous_section_inconnue_refusee() {
        assert_eq!(check(&SAMPLE.replace("### Corrigé", "### Divers")).len(), 1);
    }

    #[test]
    fn titre_de_version_mal_forme_refuse() {
        assert_eq!(
            check(&SAMPLE.replace("[0.1.0] — 2026-11-02", "[0.1] — 2026-11-02")).len(),
            1
        );
        assert_eq!(
            check(&SAMPLE.replace("— 2026-11-02", "- 2026-11-02")).len(),
            1
        );
    }

    #[test]
    fn non_publie_apres_une_version_refuse() {
        let bad = "# Changelog\n\n## [0.1.0] — 2026-11-02\n\n## [Non publié]\n";
        assert_eq!(check(bad).len(), 1);
    }
}
