extern crate sanitize_filename;

use std::fs;
use std::io::Write;
use std::str;

use actix_web::{post, web, HttpResponse, Error};
use actix_multipart::Multipart;
use futures_util::{StreamExt, TryStreamExt};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
struct Dir {
    dir: String,
}

#[post("/dir")]
async fn get_dir(dir: web::Json<Dir>) -> Result<HttpResponse, Error> {
    let paths = fs::read_dir(&dir.dir).unwrap();
    
    for p in paths {
        println!("{}", &p.unwrap().path().display());
    }

    Ok(HttpResponse::Ok().into())
}

#[post("/upload")]
async fn upload(mut payload: Multipart) -> Result<HttpResponse, Error> {
    let mut dir: String = String::from("");
    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_type = field.content_disposition().unwrap();
        let name = content_type.get_name().unwrap();
        // テキストパラメータ
        if name == "directory" {
            // バイナリ->Stringへ変換して変数に格納
            while let Some(chunk) = field.next().await {
                let data = chunk.unwrap();
                dir = match str::from_utf8(&data) {
                    Ok(s) => s.to_string().trim_end_matches("/").to_owned(),
                    Err(e) => continue
                }
            }
        // ファイルデータ
        } else if name == "file" {
            let filename = content_type.get_filename().unwrap();
            let filepath = format!("{}/{}", dir, sanitize_filename::sanitize(&filename));

            // ファイル作成
            let mut f = web::block(|| std::fs::File::create(filepath))
                .await
                .unwrap();

            // バイナリをチャンクに分けてwhileループ
            while let Some(chunk) = field.next().await {
                let data = chunk.unwrap();
                // ファイルへの書き込み
                f = web::block(move || f.write_all(&data).map(|_| f)).await?;
            }
        }
    }
    Ok(HttpResponse::Ok().into())
}