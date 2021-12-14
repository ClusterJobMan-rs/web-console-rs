use std::string::*;

use actix::prelude::*;
use actix_web::{get, web, HttpResponse};
use actix_redis::{Command as RCmd, RedisActor};
use redis_async::{resp::{RespValue, FromResp}, resp_array};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Status {
    os_type: String,
    os_release: String,
    cpu_num: u32,
    cpu_speed: u64,
    proc_total: u64,
    cpu_user: f32,
    cpu_nice: f32,
    cpu_system: f32,
    cpu_idle: f32,
    load_one: f32,
    load_five: f32,
    load_fifteen: f32,
    mem_total: u64,
    mem_free: u64,
}

impl Status {
    fn new() -> Status {
        Status {
            os_type: String::new(),
            os_release: String::new(),
            cpu_num: 0,
            cpu_speed: 0,
            proc_total: 0,
            cpu_user: 0.0,
            cpu_nice: 0.0,
            cpu_system: 0.0,
            cpu_idle: 0.0,
            load_one: 0.0,
            load_five: 0.0,
            load_fifteen: 0.0,
            mem_total: 0,
            mem_free: 0,
        }
    }
}

#[get("/status")]
async fn status(redis: web::Data<Addr<RedisActor>>) -> HttpResponse {
    let mut status: Status = Status::new();

    let addr: Vec<u8>;
    match redis.send(RCmd(resp_array!["SMEMBERS", "nodes"])).await.unwrap() {
        Ok(a) => {
            match a {
                RespValue::Array(v) => {
                    addr = FromResp::from_resp(v[0].clone()).unwrap();
                }
                _ => {
                    panic!("not array");
                }
            }
        },
        Err(e) => {
            panic!("{:?}", e);

        }
    }
    //let addr_arr = FromResp::from_resp(addr).unwrap();

    match redis.send(RCmd(resp_array!["HVALS", format!("{}_status", format!("{}", String::from_utf8(addr).unwrap()))])).await {
        Ok(err) => {
            match err {
                Ok(resp) => {
                    match resp {
                        RespValue::Array(v) => {
                            status.os_type = String::from_utf8(FromResp::from_resp(v[0].clone()).unwrap()).unwrap();
                            status.os_release = String::from_utf8(FromResp::from_resp(v[1].clone()).unwrap()).unwrap();
                            status.cpu_num = String::from_utf8(FromResp::from_resp(v[2].clone()).unwrap()).unwrap().parse::<u32>().unwrap();
                            status.cpu_speed = String::from_utf8(FromResp::from_resp(v[3].clone()).unwrap()).unwrap().parse::<u64>().unwrap();
                            status.proc_total = String::from_utf8(FromResp::from_resp(v[4].clone()).unwrap()).unwrap().parse::<u64>().unwrap();
                            status.cpu_user = String::from_utf8(FromResp::from_resp(v[5].clone()).unwrap()).unwrap().parse::<f32>().unwrap();
                            status.cpu_nice = String::from_utf8(FromResp::from_resp(v[6].clone()).unwrap()).unwrap().parse::<f32>().unwrap();
                            status.cpu_system = String::from_utf8(FromResp::from_resp(v[7].clone()).unwrap()).unwrap().parse::<f32>().unwrap();
                            status.cpu_idle = String::from_utf8(FromResp::from_resp(v[8].clone()).unwrap()).unwrap().parse::<f32>().unwrap();
                            status.load_one = String::from_utf8(FromResp::from_resp(v[9].clone()).unwrap()).unwrap().parse::<f32>().unwrap();
                            status.load_five = String::from_utf8(FromResp::from_resp(v[10].clone()).unwrap()).unwrap().parse::<f32>().unwrap();
                            status.load_fifteen = String::from_utf8(FromResp::from_resp(v[11].clone()).unwrap()).unwrap().parse::<f32>().unwrap();
                            status.mem_total = String::from_utf8(FromResp::from_resp(v[12].clone()).unwrap()).unwrap().parse::<u64>().unwrap();
                            status.mem_free = String::from_utf8(FromResp::from_resp(v[13].clone()).unwrap()).unwrap().parse::<u64>().unwrap();
                        },
                        _ => {
                            panic!("not array");
                        }
                    }
                },
                Err(e) => {
                    panic!("{:?}", e);
                }
            }
        },
        Err(e) => {
            panic!("{:?}", e);
        }
    }

    HttpResponse::Ok().json(status)
}
