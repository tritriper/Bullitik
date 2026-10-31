//! llama-server (llama.cpp) lancé en sous-processus, et son API de chat.

use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;
use serde_json::{Value, json};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

pub struct LlamaServer {
    child: Option<Child>,
    base: String,
    log: Option<PathBuf>,
    agent: ureq::Agent,
    pub startup: Duration,
}

/// Une réponse du modèle et ses temps, tels que llama-server les mesure.
#[derive(Debug)]
pub struct Completion {
    pub text: String,
    /// Temps total de la requête, vu du jeu.
    pub wall: Duration,
    pub prompt_tokens: u64,
    pub prompt_time: Duration,
    pub gen_tokens: u64,
    pub gen_tps: f64,
    /// « stop » si le modèle a fini, « length » si max_tokens l'a coupé.
    pub finish_reason: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
    timings: Option<Timings>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct Message {
    content: String,
}

#[derive(Deserialize)]
struct Timings {
    prompt_n: u64,
    prompt_ms: f64,
    predicted_n: u64,
    predicted_per_second: f64,
}

impl LlamaServer {
    /// Lance llama-server sur un port libre de 127.0.0.1 et attend qu'il soit prêt.
    pub fn start(llama_dir: &Path, model: &Path, ctx: u32, cpu: bool, log: &Path) -> Result<Self> {
        let exe = llama_dir.join(if cfg!(windows) {
            "llama-server.exe"
        } else {
            "llama-server"
        });
        ensure!(
            exe.is_file(),
            "{} introuvable (lancer `cargo xtask fetch`)",
            exe.display()
        );
        ensure!(
            model.is_file(),
            "{} introuvable (lancer `cargo xtask fetch`)",
            model.display()
        );
        let port = std::net::TcpListener::bind("127.0.0.1:0")?
            .local_addr()?
            .port();
        if let Some(dir) = log.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let out = File::create(log).with_context(|| format!("création de {}", log.display()))?;
        let mut cmd = Command::new(&exe);
        cmd.arg("--model")
            .arg(model)
            .args(["--host", "127.0.0.1", "--port", &port.to_string()])
            .args([
                "--ctx-size",
                &ctx.to_string(),
                "--parallel",
                "1",
                "--no-webui",
            ])
            // -lv 4 : le journal donne la mémoire prise par appareil.
            .args(["--log-verbosity", "4"])
            .stdin(Stdio::null())
            .stdout(out.try_clone()?)
            .stderr(out);
        if cpu {
            cmd.args(["--n-gpu-layers", "0", "--device", "none"]);
        } else {
            cmd.args(["--n-gpu-layers", "all"]);
        }
        crate::orphan::configure(&mut cmd);
        let t = Instant::now();
        let child = cmd
            .spawn()
            .with_context(|| format!("lancement de {}", exe.display()))?;
        // Sans cela, un spike tué brutalement laisserait llama-server en vie.
        crate::orphan::attach(&child).context("rattachement de llama-server au spike")?;
        let mut server = Self::with_child(Some(child), port, Some(log.to_path_buf()));
        server.wait_ready(Duration::from_secs(600))?;
        server.startup = t.elapsed();
        Ok(server)
    }

    fn with_child(child: Option<Child>, port: u16, log: Option<PathBuf>) -> Self {
        let agent = ureq::Agent::config_builder()
            // Jamais de proxy vers 127.0.0.1, même si HTTPS_PROXY est défini.
            .proxy(None)
            .timeout_global(Some(Duration::from_secs(900)))
            .http_status_as_error(false)
            .build()
            .into();
        Self {
            child,
            base: format!("http://127.0.0.1:{port}"),
            log,
            agent,
            startup: Duration::ZERO,
        }
    }

    #[cfg(test)]
    pub fn connect(port: u16) -> Self {
        Self::with_child(None, port, None)
    }

    fn wait_ready(&mut self, timeout: Duration) -> Result<()> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(child) = self.child.as_mut()
                && let Some(status) = child.try_wait()?
            {
                bail!(
                    "llama-server s'est arrêté ({status}) :\n{}",
                    self.log_tail(15)
                );
            }
            if let Ok(resp) = self.agent.get(format!("{}/health", self.base)).call()
                && resp.status() == 200
            {
                return Ok(());
            }
            ensure!(
                Instant::now() < deadline,
                "llama-server pas prêt après {timeout:?}"
            );
            std::thread::sleep(Duration::from_millis(250));
        }
    }

    /// Une requête de chat (API compatible OpenAI). `format` impose un schéma JSON.
    pub fn chat(
        &self,
        messages: &[Value],
        temperature: f64,
        max_tokens: u32,
        format: Option<&Value>,
    ) -> Result<Completion> {
        let mut body = json!({
            "messages": messages,
            "temperature": temperature,
            "max_tokens": max_tokens,
            "stream": false,
        });
        if let Some(f) = format {
            body["response_format"] = f.clone();
        }
        let t = Instant::now();
        let mut resp = self
            .agent
            .post(format!("{}/v1/chat/completions", self.base))
            .send_json(&body)
            .context("requête à llama-server")?;
        let status = resp.status();
        let text = resp.body_mut().read_to_string()?;
        let wall = t.elapsed();
        ensure!(status == 200, "llama-server a répondu {status} : {text}");
        let parsed: ChatResponse =
            serde_json::from_str(&text).context("réponse de llama-server illisible")?;
        let choice = parsed
            .choices
            .into_iter()
            .next()
            .context("réponse sans choix")?;
        let finish_reason = choice.finish_reason.unwrap_or_default();
        let content = choice.message.content;
        let timings = parsed.timings;
        Ok(Completion {
            text: content,
            wall,
            prompt_tokens: timings.as_ref().map_or(0, |t| t.prompt_n),
            prompt_time: timings.as_ref().map_or(Duration::ZERO, |t| {
                Duration::from_secs_f64(t.prompt_ms / 1000.0)
            }),
            gen_tokens: timings.as_ref().map_or(0, |t| t.predicted_n),
            gen_tps: timings.as_ref().map_or(0.0, |t| t.predicted_per_second),
            finish_reason,
        })
    }

    /// Les appareils que llama.cpp annonce utiliser (lignes « using device »).
    pub fn devices(&self) -> Result<Vec<String>> {
        Ok(self
            .log_lines()?
            .iter()
            .filter_map(|l| {
                l.split_once("using device ")
                    .map(|(_, rest)| rest.trim().to_string())
            })
            .collect())
    }

    /// Mémoire prise par appareil, d'après le dernier tableau « memory
    /// breakdown » du journal (llama-server lancé avec `-lv 4`).
    pub fn memory(&self) -> Result<Vec<DeviceMemory>> {
        Ok(parse_breakdown(&self.log_lines()?))
    }

    fn log_lines(&self) -> Result<Vec<String>> {
        match &self.log {
            // Le journal verbeux contient des octets hors UTF-8 (morceaux de tokens).
            Some(p) => Ok(String::from_utf8_lossy(&std::fs::read(p)?)
                .lines()
                .map(String::from)
                .collect()),
            None => Ok(Vec::new()),
        }
    }

    fn log_tail(&self, n: usize) -> String {
        let lines = self.log_lines().unwrap_or_default();
        lines[lines.len().saturating_sub(n)..].join("\n")
    }
}

impl Drop for LlamaServer {
    fn drop(&mut self) {
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// Une ligne du tableau « memory breakdown » de llama.cpp.
#[derive(Debug, PartialEq)]
pub struct DeviceMemory {
    pub device: String,
    /// Mémoire prise par llama.cpp sur cet appareil (modèle + contexte + calcul), en Mio.
    pub used_mib: u64,
    /// La ligne telle quelle, pour le rapport.
    pub raw: String,
}

/// Lignes du dernier tableau, de la forme
/// `|   - Host     |   2765 =  1545 +  1088 +  132   |` (appareil sans total) ou
/// `|   - Vulkan0 (…) | 12272 = 5000 + (6500 = 4400 + 1088 + 300) + 772 |`
/// (total = libre + (pris = modèle + contexte + calcul) + non compté).
fn parse_breakdown(lines: &[String]) -> Vec<DeviceMemory> {
    let Some(start) = lines
        .iter()
        .rposition(|l| l.contains("memory breakdown [MiB]"))
    else {
        return Vec::new();
    };
    lines[start + 1..]
        .iter()
        .take_while(|l| l.contains("common_memory_breakdown_print"))
        .filter_map(|l| {
            let row = l.split_once("|   - ")?.1;
            let (device, numbers) = row.split_once('|')?;
            let used_part = match numbers.split_once('(') {
                Some((_, after)) => after,
                None => numbers,
            };
            let used_mib = used_part.split_whitespace().next()?.parse().ok()?;
            Some(DeviceMemory {
                device: device.trim().to_string(),
                used_mib,
                raw: format!(
                    "{}|{}",
                    device.trim(),
                    numbers.trim_end_matches(|c: char| c == '|' || c.is_whitespace())
                ),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};

    #[test]
    fn memoire_lue_dans_le_tableau() {
        let log: Vec<String> = [
            "0.00.4 I common_memory_breakdown_print: | memory breakdown [MiB] | total   free    self   model   context   compute    unaccounted |",
            "0.00.4 I common_memory_breakdown_print: |   - Host               |                 9999 =  1 +    1 +     1                |",
            "0.11.1 I common_memory_breakdown_print: | memory breakdown [MiB] | total   free    self   model   context   compute    unaccounted |",
            "0.11.1 I common_memory_breakdown_print: |   - Vulkan0 (Radeon RX 7800 XT) | 16368 = 9000 + (6500 = 4400 +    1088 +     1012) + 868 |",
            "0.11.1 I common_memory_breakdown_print: |   - Host               |                 2765 =  1545 +    1088 +     132                |",
            "0.11.2 I common_params_fit_impl: autre chose",
        ]
        .map(String::from)
        .to_vec();
        let m = parse_breakdown(&log);
        assert_eq!(m.len(), 2, "{m:?}");
        assert_eq!(
            (m[0].device.as_str(), m[0].used_mib),
            ("Vulkan0 (Radeon RX 7800 XT)", 6500)
        );
        assert_eq!((m[1].device.as_str(), m[1].used_mib), ("Host", 2765));
        assert!(m[1].raw.starts_with("Host|"));
    }

    /// Le client contre un faux llama-server : requête envoyée, réponse et
    /// temps relus.
    #[test]
    fn chat_contre_un_faux_serveur() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let fake = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut len = 0;
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    len = v.trim().parse().unwrap();
                }
                if line == "\r\n" {
                    break;
                }
            }
            let mut body = vec![0; len];
            reader.read_exact(&mut body).unwrap();
            let request: Value = serde_json::from_slice(&body).unwrap();
            let reply = json!({
                "choices": [{"message": {"role": "assistant", "content": "{\"a_repondu\":false}"}, "finish_reason": "stop"}],
                "timings": {"prompt_n": 42, "prompt_ms": 120.0, "predicted_n": 7, "predicted_ms": 70.0, "predicted_per_second": 100.0}
            })
            .to_string();
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{reply}", reply.len()).unwrap();
            request
        });
        let server = LlamaServer::connect(port);
        let format = json!({"type": "json_schema"});
        let c = server
            .chat(
                &[json!({"role": "user", "content": "Bonjour"})],
                0.1,
                50,
                Some(&format),
            )
            .unwrap();
        let request = fake.join().unwrap();
        assert_eq!(request["response_format"]["type"], "json_schema");
        assert_eq!(request["max_tokens"], 50);
        assert_eq!(c.text, "{\"a_repondu\":false}");
        assert_eq!(c.prompt_tokens, 42);
        assert_eq!(c.prompt_time, Duration::from_millis(120));
        assert_eq!(c.gen_tokens, 7);
        assert_eq!(c.finish_reason, "stop");
    }
}
