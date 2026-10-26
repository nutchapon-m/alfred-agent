use std::sync::Arc;

use alfred_agent::{Agent, Args, Trace};
use clap::Parser;

#[tokio::main]
async fn main() {
    // Load arguments
    let args = Args::parse();

    // Initialize tracing
    if let Err(e) = Trace::init(args.log_level.clone()) {
        panic!("{}", e)
    }

    let agent = Arc::new(Agent::new(args.clone()).await);

    // Running agent
    if args.run {
        if let Err(e) = agent.run().await {
            panic!("error: {}", e)
        }
    }

    println!("{}", args.duration)
}
