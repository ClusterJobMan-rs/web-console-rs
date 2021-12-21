pub mod job;
pub mod monitor;
pub mod tcp_connector;

use crate::job::*;
use crate::monitor::*;
use crate::tcp_connector::*;

use std::collections::VecDeque;

use actix_files::NamedFile;
use actix_redis::RedisActor;
use actix_web::{get, web, App, HttpRequest, HttpServer, Result};
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use tokio::sync::Mutex;

#[get("/")]
async fn index(req: HttpRequest) -> Result<NamedFile> {
    println!("{:?}", req);
    let ua = req.headers().get("user-agent").unwrap().to_str().unwrap();
    let open_file: String;
    if ua.find("Trident") != None || ua.find("MSIE") != None {
        open_file = "static/ie.html".to_string();
    } else {
        open_file = "static/index.html".to_string();
    }
    Ok(NamedFile::open(open_file)?)
}

async fn not_found() -> Result<NamedFile> {
    Ok(NamedFile::open("static/404.html")?)
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    // load ssl keys
    // to create a self-signed temporary cert for testing:
    // `openssl req -x509 -newkey rsa:4096 -nodes -keyout key.pem -out cert.pem -days 365 -subj '/CN=localhost'`
    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
    builder
        .set_private_key_file("key.pem", SslFiletype::PEM)
        .unwrap();
    builder.set_certificate_chain_file("cert.pem").unwrap();

    STREAMS.set(Mutex::new(VecDeque::new())).unwrap();
    OUTPUTS.set(Mutex::new(VecDeque::new())).unwrap();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:33333")
        .await
        .expect("could not bind tcp socket");

    tokio::spawn(async move {
        receiver(listener).await;
    });

    HttpServer::new(|| {
        let redis_addr = RedisActor::start("127.0.0.1:6379");

        App::new()
            .data(redis_addr)
            .service(index)
            .service(get_status)
            .service(enqueue_job)
            .service(web::resource("/ws/").route(web::get().to(script_start)))
            .default_service(web::route().to(not_found))
    })
    .bind_openssl("127.0.0.1:8080", builder)?
    .run()
    .await
}
