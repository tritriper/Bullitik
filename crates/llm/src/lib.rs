//! Le modèle de langue de Bullitik : llama-server (llama.cpp) lancé en
//! sous-processus, son API de chat, et les contenus qui le pilotent (prompts,
//! fiche du juge, schéma de notation, questions).

pub mod content;
mod orphan;
pub mod server;

pub use content::{Content, Example, Notation, Question};
pub use server::{Completion, DeviceMemory, LlamaServer};

use serde_json::{Value, json};

/// Le `response_format` qui impose au juge le schéma de notation.
pub fn judge_format(content: &Content) -> Value {
    json!({
        "type": "json_schema",
        "json_schema": {"name": "notation", "strict": true, "schema": content.schema},
    })
}

/// Messages pour que l'intervieweur pose une question sur un thème.
pub fn interviewer_question(content: &Content, theme: &str) -> Vec<Value> {
    vec![
        json!({"role": "system", "content": content.interviewer}),
        json!({"role": "user", "content": format!("Thème : {theme}. Pose ta question.")}),
    ]
}

/// Messages pour que l'intervieweur relance après la réponse de l'invité.
/// Mistral exige l'alternance utilisateur / assistant après le prompt système.
pub fn interviewer_followup(
    content: &Content,
    theme: &str,
    question: &str,
    answer: &str,
) -> Vec<Value> {
    let mut messages = interviewer_question(content, theme);
    messages.push(json!({"role": "assistant", "content": question}));
    messages.push(json!({"role": "user", "content": format!("Réponse de l'invité : « {answer} »\nRelance-le.")}));
    messages
}

/// Messages pour que le juge note une réponse.
pub fn judge_messages(content: &Content, question: &str, answer: &str) -> Vec<Value> {
    vec![
        json!({"role": "system", "content": content.judge}),
        json!({"role": "user", "content": format!("Question : {question}\nRéponse du joueur : {answer}")}),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_relance_alterne_les_roles() {
        let c =
            Content::load(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"))
                .unwrap();
        let roles: Vec<String> = interviewer_followup(&c, "dette", "Q ?", "R.")
            .iter()
            .map(|m| m["role"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(roles, ["system", "user", "assistant", "user"]);
    }
}
