use anyhow::Result;
use tracing_subscriber;

mod server;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_line_number(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_target(true)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    server::run("0.0.0.0:2053")?;

    Ok(())
}
