mod messages;
mod scheduler;
mod settings;
mod task;
mod web;

use actix::prelude::*;
use dotenvy::dotenv;
use redis::aio::ConnectionManager;
use std::env;
use tracing::{debug, error, info, warn};
use tracing_error::ErrorLayer;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};


use crate::scheduler::Scheduler;
use crate::settings::Settings;
use crate::task::executor::TaskExecutor;
use crate::web::init_web_server;

fn setup_tracing() {
    use tracing_subscriber::EnvFilter;

    let env_filter = EnvFilter::from_default_env();
    let enable_color = true;//std::io::stdout().is_terminal();

    let subscriber = fmt()
        .pretty()
        .with_env_filter(env_filter)
        .with_ansi(enable_color)
        .finish()
        .with(ErrorLayer::default());

    subscriber.try_init().expect("failed to set global default subscriber");
}

#[actix_web::main]
async fn main() -> anyhow::Result<(), anyhow::Error> {
    dotenv().ok();
    setup_tracing();

    let settings = Settings::new()?;

    let redis_url = env::var("REDIS_PRIVATE_URL").expect("Missing Redis URL");
    let client = redis::Client::open(redis_url).unwrap();
    let backend = ConnectionManager::new(client).await.unwrap();

    // Create actix actors and path the reference of the task_executor to the scheduler
    // The scheduler will send it's own address in the StartTask payload for bidirectional communication
    info!("Starting task executor with {} workers", settings.workers);
    let task_executor =
        SyncArbiter::start(settings.workers, move || TaskExecutor { scheduler: None });

    let scheduler = Scheduler::new(task_executor.clone(), settings.clone());

    init_web_server(scheduler.start(), settings, backend).await?;

    Ok(())
}
