use actix_web::{
    http::Method, middleware::Logger, web, App, HttpResponse, HttpServer, Responder, Result,
};
use dotenvy::dotenv;
use redis::aio::ConnectionManager;
use std::env;

mod models {
    use redis::aio::ConnectionManager;
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize, Serialize, Clone, Debug)]
    pub struct IncomingMessage {
        #[serde(rename = "MessageSid")]
        pub message_sid: String,
        #[serde(rename = "SmsSid")]
        pub sms_id: Option<String>,
        #[serde(rename = "SmsMessageSid")]
        pub sms_message_sid: Option<String>,
        #[serde(rename = "AccountSid")]
        pub account_sid: Option<String>,
        #[serde(rename = "MessagingServiceSid")]
        pub messaging_service_sid: Option<String>,
        #[serde(rename = "From")]
        pub from: String,
        #[serde(rename = "To")]
        pub to: String,
        #[serde(rename = "Body")]
        pub body: String,
        #[serde(rename = "NumMedia")]
        pub num_media: Option<String>,
        #[serde(rename = "NumSegments")]
        pub num_segments: Option<String>,
    }

    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GlobalConfig {
        pub token: Option<String>,
        pub key: Option<String>,
    }

    #[derive(Clone)]
    pub struct AppState {
        pub redis: ConnectionManager,
        pub global_config: GlobalConfig,
    }
}

mod handlers {
    use crate::models::{GlobalConfig, IncomingMessage};
    use crate::AppState;
    use actix_web::middleware::Logger;
    use actix_web::{web, Error, HttpRequest, HttpResponse};
    use orion::util::secure_cmp;
    use redis::AsyncCommands;
    use std::time::{SystemTime, UNIX_EPOCH};

    pub async fn record_incoming_message(
        params: web::Form<IncomingMessage>,
        query: web::Query<GlobalConfig>,
        config: web::Data<AppState>,
    ) -> Result<HttpResponse, Error> {
        Logger::new("New message {params:?}");
        let mut validated = false;
        let token = config.global_config.token.clone().expect("No token found");

        if let Some(token_from_query) = query.token.clone() {
            validated = secure_cmp(token_from_query.clone().as_bytes(), token.as_bytes()).is_ok()
        }

        match validated {
            true => {
                let mut redis = config.redis.clone();
                let _: () = redis
                    .set(
                        params.message_sid.clone(),
                        format!("From: {} Body: {}", params.from, params.body),
                    )
                    .await
                    .expect("Failed to write to Redis");

                Ok(HttpResponse::Ok().content_type("text/xml").body(
                    "<?xml version=\"1.0\" encoding=\"UTF-8\"?><Response><Message></Message></Response>"
                        .to_string(),
                ))
            }
            false => {
                return Ok(HttpResponse::Unauthorized().finish());
            }
        }
    }

    pub async fn record_webhook_message(
        req_body: String,
        query: web::Query<GlobalConfig>,
        config: web::Data<AppState>,
    ) -> Result<HttpResponse, Error> {
        Logger::new("New message {req_body:?}");
        let mut validated = false;
        let token = config.global_config.token.clone().expect("No token found");

        if let Some(token_from_query) = query.token.clone() {
            validated = secure_cmp(token_from_query.clone().as_bytes(), token.as_bytes()).is_ok()
        }

        match validated {
            true => {
                let key_from_query = query.key.clone().expect("no key!");
                let sys_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
                let sys_time = format!("{}", sys_time.as_secs());
                let mut s = String::from(key_from_query);
                s.push_str("-");
                s.push_str(&sys_time);
                let mut redis = config.redis.clone();
                let _: () = redis
                    .set(
                        s,
                        &req_body,
                    )
                    .await
                    .expect("Failed to write to Redis");

                Ok(HttpResponse::Ok().content_type("text/xml").body(
                    "<?xml version=\"1.0\" encoding=\"UTF-8\"?><Response><Message></Message></Response>"
                        .to_string(),
                ))
            }
            false => {
                return Ok(HttpResponse::Unauthorized().finish());
            }
        }
    }

    pub async fn healthcheck(_req: HttpRequest) -> Result<HttpResponse, Error> {
        Ok(HttpResponse::Ok()
            .content_type("text/plain")
            .body("Hello!".to_string()))
    }
}

async fn default_handler(_req_method: Method) -> Result<impl Responder> {
    Ok(HttpResponse::MethodNotAllowed().finish())
}

use crate::models::AppState;
use handlers::{healthcheck, record_incoming_message, record_webhook_message};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let port = env::var("PORT").expect("Missing port number");
    let port = port.parse::<u16>().expect("Invalid port given");

    let secret_token = env::var("PRIVATE_EXCHANGE_TOKEN").expect("Missing port number");

    let redis_url = env::var("REDIS_PRIVATE_URL").expect("Missing Redis URL");
    let client = redis::Client::open(redis_url).unwrap();
    let backend = ConnectionManager::new(client).await.unwrap();

    let data = web::Data::new(AppState {
        redis: backend,
        global_config: models::GlobalConfig {
            token: Some(secret_token),
            key: Some(String::from("")),
        },
    });

    let server = HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(data.clone())
            .service(web::resource("/incoming").route(web::post().to(record_incoming_message)))
            .service(web::resource("/webhook").route(web::post().to(record_webhook_message)))
            .service(web::resource("/healthcheck").route(web::get().to(healthcheck)))
            .default_service(web::to(default_handler))
    })
    .bind(("0.0.0.0", port))?
    .run();

    println!("Server running at http://0.0.0.0:{}/", port);

    server.await
}
