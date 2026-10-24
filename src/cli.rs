use clap::{Parser, ValueEnum};
use std::fs;
use std::path::{Path, PathBuf};

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
    Plan,
}

impl PermissionMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AcceptEdits => "acceptEdits",
            Self::Plan => "plan",
        }
    }
}

#[derive(Parser, Clone)]
#[command(name = "alfred-agent", version = "1.0")]
pub struct Args {
    #[arg(
        long,
        help = "Runing alfred-agent to waiting for task",
        requires = "interval"
    )]
    pub run: bool,

    #[arg(long, default_value = "Read")]
    pub allowed_tools: String,

    #[arg(long, default_value = "")]
    pub disallowed_tools: String,

    #[arg(long, default_value = "50")]
    pub max_turns: String,

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

    #[arg(short, long, help = "Claude agent model", default_value = "opus")]
    pub model: String,

    #[arg(long, help = "Permission mode", value_enum, default_value_t = PermissionMode::AcceptEdits)]
    pub permission_mode: PermissionMode,

    #[arg(short, long, help = "Log level", value_enum, default_value_t = Level::INFO)]
    pub log_level: Level,

    #[arg(short, long, help = "Interval")]
    pub interval: Option<u64>,

    #[arg(long, default_value = "")]
    pub dir: String,

    #[arg(short, long)]
    pub verbose: bool,
}

impl Args {
    pub fn load_system_prompt(&self) -> Result<String, std::io::Error> {
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

    pub fn load_prompt(&self) -> Result<String, std::io::Error> {
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

    pub fn load_mcp_config(&self) -> Result<String, std::io::Error> {
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
        fs::read_to_string(path)
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
