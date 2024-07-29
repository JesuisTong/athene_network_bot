use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use reqwest::header::{HeaderMap, HeaderValue, COOKIE};
use reqwest::{StatusCode, Url};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::io::Write;
use std::time::Duration;
use std::{fs, path};
use tokio::time::sleep;
use tokio_cron_scheduler::{Job, JobScheduler, JobSchedulerError};

mod utils;

#[derive(Serialize, Deserialize, Debug)]
struct TapData {
    number_gem: f32,
    number_ec: i32,
    level: i32,
    base_rate: f32,
    min_ec: i32,
    number_tap: i64,
}

#[derive(Debug)]
enum AthenaErr {
    TapErr,
    LoginErr,
    GetMiningErr,
}

impl Display for AthenaErr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for AthenaErr {}

fn concat_str(s: i64, d: i64) -> String {
    let ts = utils::get_current_timestamp();
    format!("{s}-{ts}-{d}")
}

async fn get_tap_earn(cookie: &str) -> Result<TapData, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    // let client = reqwest::Client::builder().proxy(reqwest::Proxy::http("http://127.0.0.1:13453")?).build()?;
    let mut headers = HeaderMap::new();
    utils::init_headers(&mut headers);
    headers.insert(
        COOKIE,
        HeaderValue::from_str(&format!("token={}", cookie)).unwrap(),
    );

    let response = client
        .get("https://miniapp.athene.network/api/get-tap-earn?lang=en")
        .headers(headers)
        .send()
        .await?;

    // let response_text = response.te.await?;
    // println!("{:?}", response_text);
    let status = response.status();
    if status == StatusCode::OK {
        let d: serde_json::Value = serde_json::from_str(response.text().await?.as_str()).unwrap();
        if d["message"] == "ok" {
            return Ok(TapData {
                number_gem: d["data"]["numberGem"].as_f64().unwrap() as f32,
                number_ec: d["data"]["numberEc"].as_i64().unwrap() as i32,
                level: d["data"]["level"].as_i64().unwrap() as i32,
                base_rate: d["data"]["baseRate"].as_f64().unwrap() as f32,
                min_ec: d["data"]["minEc"].as_i64().unwrap() as i32,
                number_tap: d["data"]["numberTap"].as_i64().unwrap(),
            });
        }
    }
    //     if response.status()
    // println!("{:?}", response.text().await?);
    // let d: serde_json::Value = serde_json::from_str(response.text().await?.as_str()).unwrap();
    println!("get_tap_earn_error: {:?}", status);
    Err(Box::new(AthenaErr::TapErr))
}

async fn post_conver_gem(re: String, cookie: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    utils::init_headers(&mut headers);
    headers.insert(
        COOKIE,
        HeaderValue::from_str(&format!("token={}", cookie)).unwrap(),
    );

    let body = json!({
        "encrypt": re,
    });
    let response = client
        .post("https://miniapp.athene.network/api/post-convert-gem?lang=en")
        .headers(headers)
        .body(body.to_string())
        .send()
        .await?;

    // let response_text = response.te.await?;
    // println!("{:?}", response_text);
    println!("post-convert-gem: {:?}", response.status());
    let txt = response.text().await?;
    println!("post-convert-gem-result: {:?}", txt);
    Ok(())
}

async fn get_mining_time(cookie: &str) -> Result<i64, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    utils::init_headers(&mut headers);
    headers.insert(
        COOKIE,
        HeaderValue::from_str(&format!("token={}", cookie)).unwrap(),
    );

    let response = client
        .get("https://miniapp.athene.network/api/get-mining")
        .headers(headers)
        .send()
        .await?;

    // let response_text = response.te.await?;
    // println!("{:?}", response_text);
    println!("get_mining: {:?}", response.status());
    let status = response.status();
    if status == StatusCode::OK {
        let val: serde_json::Value = serde_json::from_str(&response.text().await?).unwrap();
        return Ok(val["data"]["remainTimeNextClaim"].as_i64().unwrap());
    }

    println!("get_mining_time_error: {:?}", status);
    Err(Box::new(AthenaErr::GetMiningErr))
}

async fn post_claim_gem(cookie: &str, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let rest_mining_time = get_mining_time(cookie).await?;
    utils::format_println(name, &format!("get_mining_time: {}", rest_mining_time));

    if rest_mining_time <= 0i64 {
        sleep(Duration::from_secs(1)).await;

        let client = reqwest::Client::new();
        // let client = reqwest::Client::builder().proxy(reqwest::Proxy::http("http://127.0.0.1:13453")?).build()?;
        let mut headers = HeaderMap::new();
        utils::init_headers(&mut headers);
        headers.insert(
            COOKIE,
            HeaderValue::from_str(&format!("token={}", cookie)).unwrap(),
        );

        let response = client
            .post("https://miniapp.athene.network/api/post-claim-gem?lang=en")
            .headers(headers)
            .body("{}")
            .send()
            .await?;

        utils::format_println(
            name,
            &format!("post_claim_gem_status: {:?}", response.status()),
        );
        let txt = response.text().await?;
        utils::format_println(name, &format!("post_claim_gem_response: {:?}", txt));
    }

    Ok(())
}

async fn post_check_in(cookie: &str, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    utils::init_headers(&mut headers);
    headers.insert(
        COOKIE,
        HeaderValue::from_str(&format!("token={}", cookie)).unwrap(),
    );

    let response = client
        .post("https://miniapp.athene.network/api/post-check-in?lang=en")
        .headers(headers)
        .body("{}")
        .send()
        .await?;

    utils::format_println(
        name,
        &format!(
            "post_check_in_status: {:?}, {:#?}",
            response.status(),
            response.url().path()
        ),
    );
    Ok(())
}

async fn post_convert_gem(cookie: &str, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let tap_data = get_tap_earn(cookie).await?;
    let total_tap = (((utils::get_current_timestamp() - tap_data.number_tap) / 100) | 0) - 100;

    utils::format_println(name, &format!("now tap count: {}", total_tap));
    if total_tap >= tap_data.min_ec as i64 {
        sleep(Duration::from_secs(1)).await;

        let txt = concat_str(tap_data.number_tap, total_tap);
        utils::format_println(
            name,
            &format!("{}, gold exchange: {}", utils::now(), total_tap),
        );
        let re = utils::rsa_encrypt(&txt);
        post_conver_gem(re, cookie).await?;
    }

    Ok(())
}

#[warn(dead_code)]
async fn login(tg_url: &str, invite_code: &str) -> Result<String, Box<dyn std::error::Error>> {
    // let
    let url = Url::parse(tg_url)?;
    let f = url.fragment();
    if let Some(f) = f {
        let v = f.split('&').nth(0).unwrap();
        let v = v.split('=').nth(1).unwrap();
        let s = urlencoding::decode(v)?;
        let token = STANDARD.encode(format!("{}&invite_code={}", s.to_string(), invite_code));
        let body = json!({
            "token": token,
        });

        let client = reqwest::Client::new();
        let mut headers = HeaderMap::new();
        utils::init_headers(&mut headers);

        let response = client
            .post("https://athene.network/api/v1.0/auth/login-telegram")
            .headers(headers.clone())
            .body(body.to_string())
            .send()
            .await?;

        if response.status() == StatusCode::OK {
            let val: serde_json::Value = serde_json::from_str(&response.text().await?).unwrap();
            let token = val["data"]["accessToken"].as_str().unwrap();

            let response = client
                .post("https://miniapp.athene.network/api/login")
                .headers(headers)
                .body(
                    json!({
                        "token": token,
                    })
                    .to_string(),
                )
                .send()
                .await?;

            if response.status() == StatusCode::OK {
                return Ok(token.to_string());
            }
        }
    }

    Err(Box::new(AthenaErr::LoginErr))
}

#[derive(Deserialize, Serialize, Debug)]
struct User {
    link: Option<String>,
    access_token: Option<String>,
    invite_code: Option<String>,
}

impl Clone for User {
    fn clone(&self) -> Self {
        User {
            link: self.link.clone(),
            access_token: self.access_token.clone(),
            invite_code: self.invite_code.clone(),
        }
    }
}

fn read_config_json(file_path: &str) -> HashMap<String, User> {
    let file = fs::File::open(file_path).unwrap();
    let reader = std::io::BufReader::new(file);
    let hashmap: HashMap<String, User> =
        serde_json::from_reader(reader).expect("Unable to parse JSON");
    hashmap
}

fn write_config_json(file_path: &str, data: &HashMap<String, User>) {
    let json_data = serde_json::to_string_pretty(data).expect("Unable to serialize data");
    let mut file = fs::File::create(file_path).expect("Unable to create file");
    file.write_all(json_data.as_bytes())
        .expect("Unable to write data to file");
}

#[tokio::main]
async fn main() -> Result<(), JobSchedulerError> {
    let sched = JobScheduler::new().await?;

    // // read user token from file
    let file_path = path::PathBuf::from(std::env::current_dir().unwrap()).join("user.json");
    println!("file_path: {:?}", file_path);
    let users = read_config_json(file_path.to_str().unwrap());
    let mut copy_users: HashMap<String, User> = HashMap::new();

    for (name, mut user) in users {
        if user.access_token.is_none() {
            let default_invite_code = "null".to_string();
            let invite_code = user.invite_code.as_ref().unwrap_or(&default_invite_code);
            let access_token = login(&user.link.as_ref().unwrap(), &invite_code)
                .await
                .unwrap();
            user.access_token = Some(access_token);
        }
        let name = name.clone();
        // update json
        copy_users.insert(name.clone().to_string(), user.clone());

        let name = name.clone();
        let token = user.access_token.clone().unwrap();
        let name1 = name.clone();
        let token1 = token.clone();
        let name2 = name.clone();
        let token2 = token.clone();
        println!("name: {}, start", &name);

        utils::format_println(&name, "post_check_in_start");
        let _ = post_check_in(&token, &name).await.map_err(|err| {
            utils::format_println(&name, &format!("post_check_in_error: {:?}", err));
        });

        utils::format_println(&name, "post_claim_gem_start");
        let _ = post_claim_gem(&token, &name).await.map_err(|err| {
            utils::format_println(&name, &format!("post_claim_gem_error: {:?}", err));
        });

        sched
            .add(
                Job::new_repeated_async(Duration::from_secs(60 * 60 * 12), move |_, _| {
                    let token = token.clone();
                    let name = name.clone();
                    Box::pin(async move {
                        sleep(Duration::from_secs(1)).await;
                        utils::format_println(&name, "post_check_in_start");
                        let _ = post_check_in(&token, &name).await.map_err(|err| {
                            utils::format_println(
                                &name,
                                &format!("post_check_in_error: {:?}", err),
                            );
                        });
                    })
                })
                .unwrap(),
            )
            .await
            .unwrap();

        sched
            .add(
                Job::new_repeated_async(Duration::from_secs(60 * 60 * 6), move |_, _| {
                    let token = token1.clone();
                    let name = name1.clone();
                    Box::pin(async move {
                        sleep(Duration::from_secs(3)).await;
                        utils::format_println(&name, "post_claim_gem_start");
                        let _ = post_claim_gem(&token, &name).await.map_err(|err| {
                            utils::format_println(
                                &name,
                                &format!("post_claim_gem_error: {:?}", err),
                            );
                        });
                    })
                })
                .unwrap(),
            )
            .await
            .unwrap();

        sched
            .add(
                Job::new_repeated_async(Duration::from_secs(60 * 2), move |_, _| {
                    let token = token2.clone();
                    let name = name2.clone();
                    Box::pin(async move {
                        sleep(Duration::from_secs(5)).await;
                        utils::format_println(&name, "post_convert_gem_start");
                        let _ = post_convert_gem(&token, &name).await.map_err(|err| {
                            utils::format_println(
                                &name,
                                &format!("post_convert_gem_error: {:?}", err),
                            );
                        });
                    })
                })
                .unwrap(),
            )
            .await
            .unwrap();
    }

    write_config_json(file_path.to_str().unwrap(), &copy_users);
    // sched.start().await?;

    // // 最多持续7天
    // sleep(Duration::from_secs(60 * 60 * 24 * 7)).await;

    Ok(())
}
