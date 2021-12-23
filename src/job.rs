use crate::tcp_connector::*;

use std::str;
use std::time::{Duration, Instant};
use std::sync::Arc;

use actix::prelude::*;
use actix::AsyncContext;
use actix_redis::{Command as RCmd, RedisActor};
use actix_web::{post, web, Error, HttpRequest, HttpResponse, Result};
use actix_web_actors::ws;
use redis_async::{resp::{RespValue, FromResp}, resp_array};
use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
//use tokio::io::AsyncReadExt;
//use tokio::net::TcpListener;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone)]
struct MyWS {
    hb: Instant,
    redis: web::Data<Addr<RedisActor>>
}

#[derive(Deserialize, Debug)]
struct JobId {
    first_id: String,
    last_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Job {
    //user: String,
    num: String,
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

        //self.recv_tcp(ctx);
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for MyWS {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        //println!("WS: {:?}", msg);
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Text(text)) => {
                self.recv_log(ctx, text);
            }
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
    fn new(redis: web::Data<Addr<RedisActor>>) -> Self {
        Self {
            hb: Instant::now(),
            redis: redis
        }
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

    fn recv_log(&self, ctx: &mut <Self as Actor>::Context, json_string: String) {
        let redis_arc = Arc::new(self.redis.clone());
        let job_id: JobId = serde_json::from_str(json_string.trim()).unwrap();
        let rec = ctx.address().recipient();
        let fut = async move {
            let res = &redis_arc.send(RCmd(resp_array!["XREAD", "STREAMS", format!("{}-{}_output", job_id.first_id, job_id.last_id), "0"])).await.unwrap();
            match res {
                Ok(RespValue::Array(a)) => {
                    while let RespValue::BulkString(line) = FromResp::from_resp(a.clone().remove(0)).unwrap() {
                        println!("{}", str::from_utf8(&line).unwrap());
                    }
                    println!("{:?}", a);
                }
                Ok(o) => println!("not array: {:?}", o),
                Err(e) => eprintln!("{:?}", e)
            }
            loop {
                /*
                let mut outputs = OUTPUTS.get().unwrap().lock().await;
                while let Some(output) = outputs.pop_front() {
                    println!("{:?}", &output);
                    rec.do_send(OutLn { line: output })
                        .expect("failed to send string");
                }
                */
            }
        };
        fut.into_actor(self).spawn(ctx);
    }
}

#[post("/enqueue")]
async fn enqueue_job(
    job: web::Json<Job>,
    redis: web::Data<Addr<RedisActor>>,
) -> Result<HttpResponse, Error> {
    let res = redis
        .send(RCmd(resp_array![
            "XADD",
            "jobStream",
            "*",
            "num",
            &job.num,
            "script",
            &job.script
        ]))
        .await?;
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

pub async fn script_start(req: HttpRequest, stream: web::Payload, redis: web::Data<Addr<RedisActor>>) -> Result<HttpResponse, Error> {
    let res = ws::start(MyWS::new(redis), &req, stream);
    res
}
