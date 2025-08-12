use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use actix_web::http::StatusCode;
use log::*;

#[get("/hello")]
async fn webhook() -> impl Responder {
    HttpResponse::Ok().body("Hello World!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("debug"));
    HttpServer::new(|| {
        App::new()
            .wrap(actix_web::middleware::Logger::default())
            .service(webhook)
    })
        .bind("127.0.0.1:8888")?
        .run()
        .await
}
