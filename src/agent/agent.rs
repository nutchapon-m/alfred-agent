use crate::model::read_stream;
use std::io::Write;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::Args;
use crate::cli::Effort;

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
                    break
                }
                msg = rx.recv() => match msg {
                    Some(true) => {
                        info!("agent started");
                        match Arc::clone(&self).handle() {
                            Ok(_) => {},
                            Err(e) => error!("error: {}", e),
                        }
                        info!("agent completed");
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

        if Arc::clone(&self).args.immediately {
            // Send trigger immediately when started.
            if tx.send(true).await.is_err() {
                return;
            }
        }

        loop {
            tokio::select! {
                _ = token.cancelled() => {
                    break
                },
                _ = tokio::time::sleep(Duration::from_secs(interval)) => {
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

        let prompt = self.args.get_prompt()?;
        if !prompt.is_empty() {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(prompt.as_bytes())?;
                drop(stdin)
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
            let mut n = 0;
            let mut bad = 0;
            if let Some(r) = read_stream(msg.as_bytes(), |_| n += 1, |_, _| bad += 1)? {
                if let Some(result) = r.result {
                    res_msg.push_str(&result.replace("\n", ""));
                };
            };
        }

        info!("claude agent output: {}", res_msg);
        Ok(())
    }

    fn build_args(&self) -> Result<Vec<String>, std::io::Error> {
        let mut args: Vec<String> = ["-p", "--output-format", "stream-json", "--verbose"]
            .into_iter()
            .map(String::from)
            .collect();

        let system_prompt = self.args.get_system_prompt()?;
        if !system_prompt.is_empty() {
            args.push("--append-system-prompt".into());
            args.push(format!("\"{}\"", system_prompt));
        }

        if let Some(allowed_tools) = self.args.allowed_tools.clone() {
            args.push("--allowedTools".into());
            args.push(allowed_tools);
        }

        if let Some(disallowed_tools) = self.args.disallowed_tools.clone() {
            args.push("--disallowedTools".into());
            args.push(disallowed_tools);
        }

        if !self.args.max_turns.is_empty() {
            args.push("--max-turns".into());
            args.push(self.args.max_turns.clone().into());
        }

        if !self.args.model.is_empty() {
            if !self.args.support_model() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "invalid model",
                ));
            }
            args.push("--model".into());
            args.push(self.args.model.clone());
        }

        let mcp_config_path = self.args.get_mcp_config()?;
        if !mcp_config_path.is_empty() {
            args.push("--mcp-config".into());
            args.push(mcp_config_path);
        }

        let settings_file = self.args.get_settings()?;
        if !settings_file.is_empty() {
            args.push("--settings".into());
            args.push(self.args.settings.clone());
        }

        if matches!(self.args.effort, Effort::Ultracode) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "effort `ultracode` is not supported by the claude CLI --effort flag",
            ));
        }
        args.push("--effort".into());
        args.push(self.args.effort.as_str().into());

        args.push("--permission-mode".into());
        args.push(self.args.permission_mode.as_str().into());

        args.push("--input-format".into());
        args.push("stream-json".into());
        Ok(args)
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
