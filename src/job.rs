use crate::tcp_connector::*;

use std::time::{Duration, Instant};
use std::str;

use actix::prelude::*;
use actix::AsyncContext;
use actix_web::{
    post, web, Error, HttpRequest, HttpResponse, Result,
};
use actix_web_actors::ws;
use actix_redis::{Command as RCmd, RedisActor};
use redis_async::{resp::RespValue, resp_array};
use serde::{Deserialize, Serialize};
//use tokio::io::AsyncReadExt;
//use tokio::net::TcpListener;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

struct MyWS {
    hb: Instant,
}

#[derive(Serialize, Deserialize, Debug)]
struct Job {
    //user: String,
    script: String,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct OutLn {
    line: String,
}
impl Handler<OutLn> for MyWS {
    type Result = ();

    fn handle(&mut self, msg: OutLn, ctx: &mut Self::Context) {
        ctx.text(msg.line);
    }
}

impl Actor for MyWS {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);

        self.recv_tcp(ctx);
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for MyWS {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError,>, ctx: &mut Self::Context) {
        //println!("WS: {:?}", msg);
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Text(text)) => ctx.text(text),
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => ctx.stop(),
        }
    }
}

impl MyWS {
    fn new() -> Self {
        Self { hb: Instant::now() }
    }

    fn hb(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                println!("Websocket Client heartbeat failed, disconnecting!");
                ctx.stop();
                return;
            }
            ctx.ping(b"");
        });
    }

    fn recv_tcp(&self, ctx: &mut <Self as Actor>::Context) {
        let rec = ctx.address().recipient();
        let fut = async move {
            loop {
                let mut outputs = OUTPUTS.get().unwrap().lock().await;
                while let Some(output) = outputs.pop_front() {
                    println!("{:?}", &output);
                    rec.do_send(OutLn { line: output }).expect("failed to send string");
                }
            }
        };
        fut.into_actor(self).spawn(ctx);
    }
}

#[post("/enqueue")]
async fn enqueue_job(job: web::Json<Job>, redis: web::Data<Addr<RedisActor>>) -> Result<HttpResponse, Error> {
    let res = redis.send(RCmd(resp_array!["XADD", "jobStream", "*", "script", &job.script])).await?;
    match res {
        Ok(RespValue::BulkString(id)) => {
            println!("{}", str::from_utf8(&id).unwrap());
            Ok(HttpResponse::Ok().body("Successfully enqueued job"))
        }
        _ => {
            println!("---->{:?}", res);
            Ok(HttpResponse::InternalServerError().finish())
        }
    }
}

pub async fn script_start(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    let res = ws::start(MyWS::new(), &req, stream);
    res
}
