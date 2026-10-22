use clap::{Parser, ValueEnum};

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

    #[arg(long, default_value = "")]
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

    #[arg(long, help = "Path to the mcp config file", default_value = "")]
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
}
