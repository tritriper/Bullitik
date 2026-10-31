//! Lecture des contenus : prompts, fiche, schéma, questions, exemples.

use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::path::Path;

pub struct Content {
    pub interviewer: String,
    pub judge: String,
    pub schema: serde_json::Value,
    pub questions: Vec<Question>,
    pub examples: Vec<Example>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Question {
    pub theme: String,
    pub question: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Example {
    pub nom: String,
    pub question: String,
    pub reponse: String,
    pub attendu: Expected,
}

/// Ce qu'un bon juge doit trouver ; un champ absent n'est pas vérifié.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Expected {
    pub a_repondu: Option<bool>,
    pub technique: Option<String>,
}

impl Expected {
    pub fn matches(&self, n: &Notation) -> bool {
        self.a_repondu.is_none_or(|a| a == n.a_repondu)
            && self.technique.as_ref().is_none_or(|t| *t == n.technique)
    }
}

/// La réponse du juge, telle que le schéma l'impose.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Notation {
    pub a_repondu: bool,
    pub pertinence: u8,
    pub discretion: u8,
    pub technique: String,
    pub explication: String,
}

impl Notation {
    /// Score sur 100, calculé par le code et non par le modèle (proposition à
    /// valider au lot 1) : une réponse donnée vaut 0 ; sinon, pertinence ×
    /// discrétion, pour récompenser l'esquive à la fois crédible et invisible.
    pub fn score(&self) -> u32 {
        if self.a_repondu {
            0
        } else {
            u32::from(self.pertinence.min(10)) * u32::from(self.discretion.min(10))
        }
    }
}

#[derive(Deserialize)]
struct QuestionFile {
    question: Vec<Question>,
}

#[derive(Deserialize)]
struct ExampleFile {
    exemple: Vec<Example>,
}

impl Content {
    pub fn load(dir: &Path) -> Result<Self> {
        let read = |p: &str| {
            std::fs::read_to_string(dir.join(p))
                .with_context(|| format!("lecture de {}", dir.join(p).display()))
        };
        let fiche = read("juge/fiche-techniques.md")?;
        let content = Content {
            interviewer: read("prompts/intervieweur.md")?,
            judge: read("prompts/juge.md")?.replace("{{fiche}}", &fiche),
            schema: serde_json::from_str(&read("juge/schema-notation.json")?)
                .context("schema-notation.json")?,
            questions: toml::from_str::<QuestionFile>(&read("questions/questions.toml")?)
                .context("questions.toml")?
                .question,
            examples: toml::from_str::<ExampleFile>(&read("juge/exemples.toml")?)
                .context("exemples.toml")?
                .exemple,
        };
        content.check(&fiche)?;
        Ok(content)
    }

    /// Cohérence entre fichiers : de quoi détecter une faute de frappe dans un
    /// contenu avant de lancer le modèle.
    fn check(&self, fiche: &str) -> Result<()> {
        ensure!(
            !self.judge.contains("{{"),
            "prompt du juge : variable non remplacée"
        );
        ensure!(!self.questions.is_empty(), "aucune question");
        ensure!(self.examples.len() >= 2, "il faut au moins deux exemples");
        let techniques = self.techniques();
        ensure!(!techniques.is_empty(), "schéma : liste des techniques vide");
        for t in &techniques {
            ensure!(
                fiche.contains(&format!("| {t} |")),
                "technique « {t} » du schéma absente de la fiche"
            );
        }
        for ex in &self.examples {
            if let Some(t) = &ex.attendu.technique {
                ensure!(
                    techniques.contains(t),
                    "exemple « {} » : technique inconnue « {t} »",
                    ex.nom
                );
            }
        }
        Ok(())
    }

    pub fn techniques(&self) -> Vec<String> {
        self.schema["properties"]["technique"]["enum"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_content() -> Content {
        Content::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap()
    }

    #[test]
    fn les_contenus_du_depot_sont_coherents() {
        let c = repo_content();
        assert!(
            c.judge.contains("ignorer la question"),
            "la fiche doit être injectée dans le prompt du juge"
        );
        assert_eq!(c.schema["additionalProperties"], false);
    }

    #[test]
    fn score_nul_si_le_joueur_a_repondu() {
        let n = |a, p, d| Notation {
            a_repondu: a,
            pertinence: p,
            discretion: d,
            technique: "aucune".into(),
            explication: String::new(),
        };
        assert_eq!(n(true, 10, 10).score(), 0);
        assert_eq!(n(false, 9, 8).score(), 72);
        assert_eq!(n(false, 10, 0).score(), 0);
    }

    #[test]
    fn notation_avec_champ_inconnu_refusee() {
        let json = r#"{"a_repondu":false,"pertinence":5,"discretion":5,"technique":"aucune","explication":"x","note":3}"#;
        assert!(serde_json::from_str::<Notation>(json).is_err());
    }

    #[test]
    fn attendu_partiel() {
        let n: Notation = serde_json::from_str(
            r#"{"a_repondu":false,"pertinence":5,"discretion":5,"technique":"langue de bois","explication":"x"}"#,
        )
        .unwrap();
        assert!(
            Expected {
                a_repondu: Some(false),
                technique: None
            }
            .matches(&n)
        );
        assert!(
            !Expected {
                a_repondu: Some(true),
                technique: None
            }
            .matches(&n)
        );
        assert!(
            !Expected {
                a_repondu: None,
                technique: Some("aucune".into())
            }
            .matches(&n)
        );
    }
}
