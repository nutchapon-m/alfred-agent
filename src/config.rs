use dotenvy::dotenv;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(default = "default_system_prompt")]
    pub agent_system_prompt: String,
    #[serde(default = "default_prompt")]
    pub agent_prompt: String,
    #[serde(default = "default_duration")]
    pub agent_duration: u16,
}

impl Config {
    pub fn init() -> Result<Config, envy::Error> {
        dotenv().ok();
        return envy::from_env();
    }
}

fn default_system_prompt() -> String {
    return "".to_string();
}
fn default_prompt() -> String {
    return "".to_string();
}
fn default_duration() -> u16 {
    return 60;
}
