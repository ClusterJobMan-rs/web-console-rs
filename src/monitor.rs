use std::collections::HashMap;
use std::string::*;

use actix::prelude::*;
use actix_redis::{Command as RCmd, RedisActor};
use actix_web::{get, web, HttpResponse};
use redis_async::{
    resp::{FromResp, RespValue},
    resp_array,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct Nodes {
    nodes: HashMap<String, Status>,
}

impl Nodes {
    fn new() -> Nodes {
        Nodes {
            nodes: HashMap::new(),
        }
    }

    fn add(&mut self, addr: String, status: Status) {
        self.nodes.insert(addr, status);
    }
}

#[derive(Serialize, Deserialize)]
struct Status {
    hostname: String,
    os_type: String,
    os_release: String,
    cpu_num: u32,
    cpu_speed: u64,
    proc_total: u64,
    cpu_usage: HashMap<String, f32>,
    load_one: f32,
    load_five: f32,
    load_fifteen: f32,
    mem_total: u64,
    mem_free: u64,
}

impl Status {
    fn new() -> Status {
        Status {
            hostname: String::new(),
            os_type: String::new(),
            os_release: String::new(),
            cpu_num: 0,
            cpu_speed: 0,
            proc_total: 0,
            cpu_usage: HashMap::new(),
            load_one: 0.0,
            load_five: 0.0,
            load_fifteen: 0.0,
            mem_total: 0,
            mem_free: 0,
        }
    }
}

#[get("/status")]
async fn get_status(redis: web::Data<Addr<RedisActor>>) -> HttpResponse {
    let mut nodes: Nodes = Nodes::new();

    match redis
        .send(RCmd(resp_array!["SMEMBERS", "nodes"]))
        .await
        .unwrap()
    {
        Ok(a) => match a {
            RespValue::Array(mut v) => {
                while let Some(addr) = v.pop() {
                    match FromResp::from_resp(addr) {
                        Ok(s) => match String::from_utf8(s) {
                            Ok(addr) => {
                                let mut status = Status::new();
                                match redis
                                    .send(RCmd(resp_array![
                                        "HVALS",
                                        format!("{}_status", format!("{}", addr))
                                    ]))
                                    .await
                                {
                                    Ok(err) => match err {
                                        Ok(resp) => match resp {
                                            RespValue::Array(v) => {
                                                status.os_type = String::from_utf8(
                                                    FromResp::from_resp(v[0].clone()).unwrap(),
                                                )
                                                .unwrap();
                                                status.os_release = String::from_utf8(
                                                    FromResp::from_resp(v[1].clone()).unwrap(),
                                                )
                                                .unwrap();
                                                status.cpu_num = String::from_utf8(
                                                    FromResp::from_resp(v[2].clone()).unwrap(),
                                                )
                                                .unwrap()
                                                .parse::<u32>()
                                                .unwrap();
                                                status.cpu_speed = String::from_utf8(
                                                    FromResp::from_resp(v[3].clone()).unwrap(),
                                                )
                                                .unwrap()
                                                .parse::<u64>()
                                                .unwrap();
                                                status.proc_total = String::from_utf8(
                                                    FromResp::from_resp(v[4].clone()).unwrap(),
                                                )
                                                .unwrap()
                                                .parse::<u64>()
                                                .unwrap();
                                                status.load_one = String::from_utf8(
                                                    FromResp::from_resp(v[9].clone()).unwrap(),
                                                )
                                                .unwrap()
                                                .parse::<f32>()
                                                .unwrap();
                                                status.load_five = String::from_utf8(
                                                    FromResp::from_resp(v[10].clone()).unwrap(),
                                                )
                                                .unwrap()
                                                .parse::<f32>()
                                                .unwrap();
                                                status.load_fifteen = String::from_utf8(
                                                    FromResp::from_resp(v[11].clone()).unwrap(),
                                                )
                                                .unwrap()
                                                .parse::<f32>()
                                                .unwrap();
                                                status.mem_total = String::from_utf8(
                                                    FromResp::from_resp(v[12].clone()).unwrap(),
                                                )
                                                .unwrap()
                                                .parse::<u64>()
                                                .unwrap();
                                                status.mem_free = String::from_utf8(
                                                    FromResp::from_resp(v[13].clone()).unwrap(),
                                                )
                                                .unwrap()
                                                .parse::<u64>()
                                                .unwrap();
                                            }
                                            _ => {
                                                panic!("not array");
                                            }
                                        },
                                        Err(e) => {
                                            panic!("{:?}", e);
                                        }
                                    },
                                    Err(e) => {
                                        panic!("{:?}", e);
                                    }
                                }
                                match redis
                                    .send(RCmd(resp_array![
                                        "HGETALL",
                                        format!("{}_cpu_usage", format!("{}", addr))
                                    ]))
                                    .await
                                {
                                    Ok(err) => match err {
                                        Ok(resp) => match resp {
                                            RespValue::Array(arr) => {
                                                for i in 0..arr.len()/2 {
                                                    status.cpu_usage.insert(
                                                        String::from_utf8(
                                                            FromResp::from_resp(v[i*2].clone()).unwrap(),
                                                        )
                                                        .unwrap(),
                                                        String::from_utf8(
                                                            FromResp::from_resp(v[i*2+1].clone()).unwrap(),
                                                        )
                                                        .unwrap()
                                                        .parse::<f32>()
                                                        .unwrap()
                                                    );
                                                }
                                            }
                                            _ => panic!("not array")
                                        }
                                        Err(e) => panic!("{:?}", e)
                                    }
                                    Err(e) => panic!("{:?}", e)
                                }
                                nodes.add(addr, status);
                            }
                            Err(e) => {
                                panic!("{:?}", e);
                            }
                        },
                        Err(e) => {
                            panic!("{:?}", e);
                        }
                    }
                }
            }
            _ => {
                panic!("not array");
            }
        },
        Err(e) => {
            panic!("{:?}", e);
        }
    }

    HttpResponse::Ok().json(nodes)
}
