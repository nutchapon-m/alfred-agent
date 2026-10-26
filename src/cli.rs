use clap::{Parser, ValueEnum};
use std::fs;
use std::path::{Path, PathBuf};

// ===========================================================================================

#[derive(Debug, Clone, ValueEnum)]
pub enum Level {
    DEBUG,
    INFO,
    WARN,
    ERROR,
    TRACE,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum PermissionMode {
    AcceptEdits,
    BypassPermissions,
    Plan,
}

impl PermissionMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AcceptEdits => "acceptEdits",
            Self::BypassPermissions => "bypassPermissions",
            Self::Plan => "plan",
        }
    }
}

#[derive(Debug, Clone, ValueEnum)]
pub enum Effort {
    Low,
    Medium,
    High,
    Xhigh,
    Max,
    Ultracode,
}

impl Effort {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Xhigh => "xhigh",
            Self::Max => "max",
            Self::Ultracode => "ultracode",
        }
    }
}

// ===========================================================================================

#[derive(Parser, Clone)]
#[command(name = "alfred-agent", version = "1.0")]
pub struct Args {
    #[arg(
        long,
        help = "Runing alfred-agent to waiting for task",
        requires = "interval"
    )]
    pub run: bool,

    #[arg(long)]
    pub allowed_tools: Option<String>,

    #[arg(long)]
    pub disallowed_tools: Option<String>,

    #[arg(long, default_value = "50")]
    pub max_turns: String,

    #[arg(long, help = "Replace the entire system prompt with custom text")]
    pub system_prompt: Option<String>,

    #[arg(long, help = "Replace the entire prompt with custom text")]
    pub prompt: Option<String>,

    #[arg(
        long,
        help = "Path to the system prompt json file",
        default_value = "~/.alfred/system_prompt.txt"
    )]
    pub system_prompt_file: String,

    #[arg(
        long,
        help = "Path to the prompt json file",
        default_value = "~/.alfred/prompt.json"
    )]
    pub prompt_file: String,

    #[arg(
        long,
        help = "Path to the mcp config file",
        default_value = "~/.alfred/mcp_config.json"
    )]
    pub mcp_config: String,

    #[arg(long, default_value = "~/.alfred/settings.json")]
    pub settings: String,

    #[arg(short, long, help = "Claude agent model", default_value = "opus")]
    pub model: String,

    #[arg(
        long,
        help = "Set the effort level for the current session.",
        value_enum,
        default_value_t = Effort::Medium,
    )]
    pub effort: Effort,

    #[arg(short, long, help = "Permission mode", value_enum, default_value_t = PermissionMode::AcceptEdits)]
    pub permission_mode: PermissionMode,

    #[arg(short, long, help = "Log level", value_enum, default_value_t = Level::INFO)]
    pub log_level: Level,

    #[arg(short, long, help = "Interval")]
    pub interval: Option<u64>,

    #[arg(
        short,
        long,
        help = "Enable verbose logging, shows full turn-by-turn output."
    )]
    pub verbose: bool,

    #[arg(long, help = "Start agent to do work immediately.")]
    pub immediately: bool,
}

impl Args {
    pub fn get_system_prompt(&self) -> Result<String, std::io::Error> {
        if let Some(sp) = self.system_prompt.clone() {
            return Ok(sp);
        }

        let path: &Path = self.system_prompt_file.as_ref();
        let path = expand_tilde(path);
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

    pub fn get_prompt(&self) -> Result<String, std::io::Error> {
        if let Some(p) = self.prompt.clone() {
            let p = serde_json::json!({"type": "user", "message": {"role": "user", "content": p}});
            return Ok(p.to_string());
        }

        let path: &Path = self.prompt_file.as_ref();
        let path = expand_tilde(path);
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

    pub fn get_mcp_config(&self) -> Result<String, std::io::Error> {
        let path: &Path = self.mcp_config.as_ref();
        let path = expand_tilde(path);
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

        Ok(self.mcp_config.clone())
    }

    pub fn get_settings(&self) -> Result<String, std::io::Error> {
        let path: &Path = self.settings.as_ref();
        let path = expand_tilde(path);
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
        Ok(self.settings.clone())
    }

    pub fn support_model(&self) -> bool {
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

        support_models.contains(&self.model)
    }
}

fn expand_tilde(p: &Path) -> PathBuf {
    if let Ok(rest) = p.strip_prefix("~") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    p.to_path_buf()
}
