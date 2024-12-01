mod messages;
mod scheduler;
mod settings;
mod task;
mod web;

use ::actix::prelude::*;
use dotenvy::dotenv;
use log::info;
use redis::aio::ConnectionManager;
use std::env;

use crate::scheduler::Scheduler;
use crate::settings::Settings;
use crate::task::executor::TaskExecutor;
use crate::web::init_web_server;

fn main() -> Result<()> {
    dotenv().ok();

    let system = System::new();
    let settings = Settings::new()?;

    let redis_url = env::var("REDIS_PRIVATE_URL").expect("Missing Redis URL");
    let client = redis::Client::open(redis_url).unwrap();
    let backend = client.get_connection()?;

    // Create actix actors and path the reference of the task_executor to the scheduler
    // The scheduler will send it's own address in the StartTask payload for bidirectional communication
    info!("Starting task executor with {} workers", settings.workers);
    let task_executor =
        SyncArbiter::start(settings.workers, move || TaskExecutor { scheduler: None });

    let scheduler = Scheduler::new(task_executor.clone(), settings.clone());

    init_web_server(scheduler.start(), settings, backend)?;

    let _ = system.run();

    Ok(())
}
