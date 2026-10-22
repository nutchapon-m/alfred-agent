use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use std::{fs, io::Write};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::Args;

#[derive(Clone)]
pub struct Agent {
    pub args: Args,
}

impl Agent {
    pub async fn new(args: Args) -> Self {
        Self { args: args }
    }

    pub async fn run(self: Arc<Self>) -> std::io::Result<()> {
        let (tx, rx) = mpsc::channel::<bool>(1);
        let token = CancellationToken::new();

        let worker = tokio::spawn(Arc::clone(&self).future(token.clone(), rx));
        let trigger = tokio::spawn(Arc::clone(&self).trigger(token.clone(), tx));

        Arc::clone(&self).intro();

        tokio::signal::ctrl_c().await?;
        token.cancel();
        worker.await?;
        trigger.await?;
        info!("agent stopped");
        Ok(())
    }

    async fn future(self: Arc<Self>, token: CancellationToken, mut rx: mpsc::Receiver<bool>) {
        loop {
            tokio::select! {
                _ = token.cancelled() => {
                    info!("worker cancelled");
                    break
                }
                msg = rx.recv() => match msg {
                    Some(true) => {
                        match Arc::clone(&self).handle() {
                            Ok(_) => {},
                            Err(e) => error!("error: {}", e),
                        }
                    },
                    Some(false) => {},
                    None => {
                        info!("channel closed");
                        break
                    }
                }
            }
        }
    }

    async fn trigger(self: Arc<Self>, token: CancellationToken, tx: mpsc::Sender<bool>) {
        let mut interval: u64 = 0;
        if let Some(i) = Arc::clone(&self).args.interval {
            interval = i;
        }

        loop {
            tokio::select! {
                _ = token.cancelled() => {
                    info!("trigger cancelled");
                    break
                },
                _ = tokio::time::sleep(Duration::from_secs(interval)) => {
                    info!("trigger!!!");
                    if tx.send(true).await.is_err() {
                        break
                    }
                },
            }
        }
    }

    fn handle(&self) -> Result<(), std::io::Error> {
        let args = self.build_args()?;
        let mut child = std::process::Command::new("claude")
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let prompt = self.load_prompt()?;
        if !prompt.is_empty() {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(prompt.as_bytes())?;
            }
        }

        let o = child.wait_with_output()?;

        if !o.status.success() {
            return Err(std::io::Error::other(format!(
                "error: status {:?}, {}",
                o.status.code(),
                String::from_utf8_lossy(&o.stderr)
            )));
        }

        let out_string = String::from_utf8_lossy(&o.stdout);
        let mut res_msg = String::new();
        for msg in out_string.split("\n") {
            let msg = format!("\n{}", msg);
            res_msg.push_str(&msg);
        }
        info!("claude agent output:\n{}", res_msg);
        Ok(())
    }

    fn load_system_prompt(&self) -> Result<String, std::io::Error> {
        let path: &Path = self.args.system_prompt_file.as_ref();
        let path = self.expand_tilde(path);
        let exist = path.try_exists()?;
        if !exist {
            return Ok("".to_string());
        }
        // validate ext of file
        let is_txt = path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("txt"));

        if !is_txt {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("invalid file type: {} (expected .txt)", path.display()),
            ));
        }
        fs::read_to_string(path)
    }

    fn load_prompt(&self) -> Result<String, std::io::Error> {
        let path: &Path = self.args.prompt_file.as_ref();
        let path = self.expand_tilde(path);
        let exist = path.try_exists()?;
        if !exist {
            return Ok("".to_string());
        }
        // validate ext of file
        let is_txt = path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("json"));

        if !is_txt {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("invalid file type: {} (expected .json)", path.display()),
            ));
        }
        fs::read_to_string(path)
    }

    fn load_mcp_config(&self) -> Result<String, std::io::Error> {
        let path: &Path = self.args.mcp_config.as_ref();
        let path = self.expand_tilde(path);
        let exist = path.try_exists()?;
        if !exist {
            return Ok("".to_string());
        }
        // validate ext of file
        let is_txt = path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("json"));

        if !is_txt {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("invalid file type: {} (expected .json)", path.display()),
            ));
        }
        fs::read_to_string(path)
    }

    fn valid_model(&self) -> bool {
        let support_models: Vec<String> = [
            "claude-opus-5-5",
            "claude-sonnet-5-5",
            "claude-haiku-5-5",
            "claude-opus-5",
            "claude-sonnet-5",
            "claude-opus-4-8",
            "claude-sonnet-4-6",
            "claude-haiku-4-5",
            "opus",
            "sonnet",
            "haiku",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        support_models.contains(&self.args.model)
    }

    fn build_args(&self) -> Result<Vec<String>, std::io::Error> {
        info!("{}", self.args.prompt_file);
        let mut args: Vec<String> = ["-p", "--output-format", "stream-json"]
            .into_iter()
            .map(String::from)
            .collect();
        if !self.args.system_prompt_file.is_empty() {
            let system_prompt = self.load_system_prompt()?;
            args.push("--append-system-prompt".into());
            args.push(format!("\"{}\"", system_prompt));
        }
        if !self.args.allowed_tools.is_empty() {
            args.push("--allowedTools".into());
            args.push(self.args.allowed_tools.clone().into());
        }
        if !self.args.disallowed_tools.is_empty() {
            args.push("--disallowedTools".into());
            args.push(self.args.disallowed_tools.clone().into());
        }
        if !self.args.max_turns.is_empty() {
            args.push("--max-turns".into());
            args.push(self.args.max_turns.clone().into());
        }
        args.push("--verbose".into());

        if !self.args.mcp_config.is_empty() {
            let mcp_config = self.load_mcp_config()?;
            args.push("--mcp-config".into());
            args.push(format!("'{}'", mcp_config));
        }
        if !self.args.model.is_empty() {
            if !self.valid_model() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "invalid model",
                ));
            }
            args.push("--model".into());
            args.push(self.args.model.clone());
        }

        args.push("--permission-mode".into());
        args.push(self.args.permission_mode.as_str().into());

        args.push("--input-format".into());
        args.push("stream-json".into());

        info!("{:?}", args);

        Ok(args)
    }

    fn expand_tilde(&self, p: &Path) -> PathBuf {
        if let Ok(rest) = p.strip_prefix("~") {
            if let Some(home) = dirs::home_dir() {
                return home.join(rest);
            }
        }
        p.to_path_buf()
    }

    fn intro(&self) {
        info!(
            "
=====================================
▄████▄ ▄▄    ▄▄▄▄▄ ▄▄▄▄  ▄▄▄▄▄ ▄▄▄▄  
██▄▄██ ██    ██▄▄  ██▄█▄ ██▄▄  ██▀██ 
██  ██ ██▄▄▄ ██    ██ ██ ██▄▄▄ ████▀ 
=====================================

version = 1.0
program started...
"
        );
    }
}
