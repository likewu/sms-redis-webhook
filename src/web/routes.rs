use actix_web::http::Method;
use actix_web::*;
use actix_web::{http, HttpRequest, HttpResponse};
use log::{debug, info};

use orion::util::secure_cmp;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::messages::GetQueue;
use crate::web::authentication::verify_authentication_header;
use crate::web::helper::*;
use crate::web::{QueryInfo, AppState, Payload};

/// Index route for getting current state of the server
pub async fn index(data: web::Data<AppState>, request: HttpRequest) -> HttpResponse {
    let headers = match get_headers_hash_map(request.headers()) {
        Ok(headers) => headers,
        Err(response) => return response,
    };

    // Check the credentials and signature headers of the request
    if let Err(response) = verify_authentication_header(&data.settings, &headers, &Vec::new()) {
        return response;
    };

    match data.scheduler.send(GetQueue {}).await {
        Ok(json) => HttpResponse::Ok()
            .append_header((http::header::CONTENT_TYPE, "application/json"))
            .body(json),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

pub async fn webhook(
    data: web::Data<AppState>,
    path_info: web::Path<String>,
    request: HttpRequest,
    body: web::Bytes,
    //query: web::Query<QueryInfo>,
) -> HttpResponse {
    let body: Vec<u8> = body.to_vec();

    let mut validated = false;
    let token = data.settings.secret.clone().expect("No token found");
    if let Some(token_from_query) = query.token.clone() {
        validated = secure_cmp(token_from_query.clone().as_bytes(), token.as_bytes()).is_ok();
    }

    if let Some(key_from_query) = query.key.clone() {
        validated = validated && true;
    } else {
        validated = false;
    }

    match validated {
        true => {
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

            return HttpResponse::Ok().finish();
        }
        false => {
            return Ok(HttpResponse::Unauthorized().finish());
        }
    }

    let payload = match request.method() {
        &Method::POST => match get_payload(&body) {
            Ok(payload) => payload,
            Err(response) => return response,
        },
        _ => Payload::default(),
    };

    let headers = match get_headers_hash_map(request.headers()) {
        Ok(headers) => headers,
        Err(response) => return response,
    };

    let webhook_name = path_info.into_inner();

    // Check the credentials and signature headers of the request
    //if let Err(response) = verify_authentication_header(&data.settings, &headers, &body) {
    //    return response;
    //};

    info!("Incoming webhook for \"{}\":", webhook_name);
    debug!("Got payload: {:?}", payload);

    // Create a new task with the checked parameters and webhook name
    let new_task = match get_task_from_request(&data.settings, webhook_name, payload.parameters) {
        Ok(task) => task,
        Err(response) => return response,
    };

    // Send the task to the actor managing the queue
    data.scheduler.do_send(new_task);

    HttpResponse::Ok().finish()
}

pub async fn healthcheck(_req: HttpRequest) -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok()
        .content_type("text/plain")
        .body("Hello!".to_string()))
}