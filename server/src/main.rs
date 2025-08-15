use crate::api::{PushRequest, PushResponse};
use actix_web::{
    App, HttpResponse, HttpServer, Responder, Result, get, http::StatusCode, post, web,
};
use common::{PlatformRegistry, PushResult};
use log::*;
use wxwork_group_bot::WxWorkPlatformFactory;

mod api;

#[get("/hello")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello World!")
}

#[post("/push")]
async fn push(req: web::Json<PushRequest>, registry: web::Data<PlatformRegistry>) -> HttpResponse {
    info!("Received push request for platform: {}", req.platform);

    let factory = match registry.get_factory(&req.platform) {
        Some(f) => f,
        None => {
            let err_resp = PushResponse {
                result: PushResult {
                    success: false,
                    response: Some(format!("Platform '{}' not found", req.platform)),
                    ..Default::default()
                },
            };
            return HttpResponse::BadRequest().json(err_resp);
        }
    };

    let platform = match factory.create(req.config.clone()) {
        Ok(p) => p,
        Err(e) => {
            let err_resp = PushResponse {
                result: PushResult {
                    success: false,
                    response: Some(format!("Failed to create platform: {}", e)),
                    ..Default::default()
                },
            };
            return HttpResponse::BadRequest().json(err_resp);
        }
    };

    let result = platform.send(req.message.clone()).await;

    let response = match result {
        Ok(push_result) => PushResponse {
            result: push_result,
        },
        Err(push_error) => PushResponse {
            result: PushResult {
                success: false,
                response: Some(push_error.to_string()),
                ..Default::default()
            },
        },
    };

    HttpResponse::Ok().json(response)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("debug"));

    let mut registry = PlatformRegistry::new();
    registry.register(Box::new(WxWorkPlatformFactory));
    info!("Registered platforms: {:?}", registry.list_platforms());

    let registry_data = web::Data::new(registry);

    HttpServer::new(move || {
        App::new()
            .wrap(actix_web::middleware::Logger::default())
            .app_data(registry_data.clone())
            .service(hello)
            .service(push)
    })
    .bind("127.0.0.1:8888")?
    .run()
    .await
}
