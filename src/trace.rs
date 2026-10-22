use tracing::subscriber::SetGlobalDefaultError;
use tracing_subscriber::FmtSubscriber;

use crate::Level;

pub struct Trace {}

impl Trace {
    pub fn init(level: Level) -> Result<(), SetGlobalDefaultError> {
        let subscriber = FmtSubscriber::builder()
            .with_file(true)
            .with_max_level(handle_level(level))
            .finish();
        tracing::subscriber::set_global_default(subscriber)?;
        Ok(())
    }
}

fn handle_level(level: Level) -> tracing::Level {
    match level {
        Level::DEBUG => tracing::Level::DEBUG,
        Level::INFO => tracing::Level::INFO,
        Level::WARN => tracing::Level::WARN,
        Level::ERROR => tracing::Level::ERROR,
        Level::TRACE => tracing::Level::TRACE,
    }
}
