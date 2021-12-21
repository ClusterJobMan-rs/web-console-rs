use std::collections::VecDeque;
use std::net::SocketAddr;
use std::str;

use once_cell::sync::OnceCell;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

pub static MASTER_ADDR: OnceCell<Mutex<Vec<SocketAddr>>> = OnceCell::new();
pub static STREAMS: OnceCell<Mutex<VecDeque<TcpStream>>> = OnceCell::new();
pub static OUTPUTS: OnceCell<Mutex<VecDeque<String>>> = OnceCell::new();

/*
pub async fn init() {
    STREAMS.set(Mutex::new(Vec::new())).unwrap();

    let listener = TcpListener::bind("127.0.0.1:33333").await.expect("could not bind tcp socket");

    receiver(listener);
}
*/

pub async fn receiver(mut listener: TcpListener) {
    let addr = SocketAddr::from(([127, 0, 0, 1], 33333));
    let mut v = Vec::new();
    v.push(addr);
    MASTER_ADDR.set(Mutex::new(v)).unwrap();
    loop {
        let mut streams = STREAMS.get().unwrap().lock().await;
        match listener.accept().await {
            Ok((mut stream, _addr)) => {
                //if MASTER_ADDR.get().unwrap().lock().await.contains(&addr) {
                if true {
                    println!("a");
                    tokio::spawn(async move {
                        println!("b");
                        let mut buf = [0u8; 2048];
                        stream.read(&mut buf).await.expect("could not read buffer");
                        let mut outputs = OUTPUTS.get().unwrap().lock().await;
                        outputs.push_back(str::from_utf8(&buf).unwrap().to_string());
                    });
                } else {
                    println!("A");
                    println!("{:?}", &stream);
                    streams.push_back(stream);
                }
            }
            Err(e) => {
                eprintln!("failed to read buffer: {:?}", e);
            }
        };
    }
}
