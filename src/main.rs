#![deny(clippy::all)]
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use reqwest::header::{HeaderMap, HeaderValue, COOKIE};
use reqwest::{Client, StatusCode, Url};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::io::Write;
use std::sync::Arc;
use std::time::Duration;
use std::fs;
use tokio::time::sleep;

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
    Tap,
    Login,
    GetMining,
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

#[derive(Deserialize, Serialize, Debug)]
struct User {
    link: Option<String>,
    access_token: Option<String>,
    invite_code: Option<String>,
    name: Option<String>,
}

impl Clone for User {
    fn clone(&self) -> Self {
        User {
            link: self.link.clone(),
            access_token: self.access_token.clone(),
            invite_code: self.invite_code.clone(),
            name: self.name.clone(),
        }
    }
}

impl User {
    fn request_with_token(&self) -> Client {
        let mut headers = HeaderMap::new();
        utils::init_headers(&mut headers);
        headers.insert(
            COOKIE,
            HeaderValue::from_str(&format!("token={}", self.access_token.as_ref().unwrap()))
                .unwrap(),
        );

        reqwest::ClientBuilder::new()
            .default_headers(headers)
            .build()
            .unwrap()
    }

    #[warn(dead_code)]
    async fn login(&mut self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // let
        let url = Url::parse(self.link.as_ref().unwrap())?;
        let f = url.fragment();
        if let Some(f) = f {
            let v = f.split('&').nth(0).unwrap();
            let v = v.split('=').nth(1).unwrap();
            let s = urlencoding::decode(v)?;
            let token = STANDARD.encode(format!(
                "{}&invite_code={}",
                s,
                self.invite_code.clone().unwrap_or("null".to_string())
            ));
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

            let status = response.status();
            if status == StatusCode::OK {
                let val: serde_json::Value = serde_json::from_str(&response.text().await?).unwrap();
                let token = val["data"]["accessToken"].as_str().unwrap();

                client
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

                if status == StatusCode::OK {
                    self.access_token = Some(token.to_string());
                    return Ok(token.to_string());
                }
            }
            log::error!("login failed {:?}", status);
        }

        Err(Box::new(AthenaErr::Login))
    }

    async fn post_check_in(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let client = self.request_with_token();
        let name = self.name.as_ref().unwrap();

        let response = client
            .post("https://miniapp.athene.network/api/post-check-in?lang=en")
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

    async fn get_tap_earn(&self) -> Result<TapData, Box<dyn std::error::Error + Send + Sync>> {
        let client = self.request_with_token();
        let name = self.name.as_ref().unwrap();
        let response = client
            .get("https://miniapp.athene.network/api/get-tap-earn?lang=en")
            .send()
            .await?;

        // let response_text = response.te.await?;
        // println!("{:?}", response_text);
        let status = response.status();
        if status == StatusCode::OK {
            let d: serde_json::Value =
                serde_json::from_str(response.text().await?.as_str()).unwrap();
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

        utils::format_error(name, &format!("get_tap_earn_error: {:?}", status));
        Err(Box::new(AthenaErr::Tap))
    }

    async fn post_conver_gem(
        &self,
        re: String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let client = self.request_with_token();
        let name = self.name.as_ref().unwrap();

        let body = json!({
            "encrypt": re,
        });
        let response = client
            .post("https://miniapp.athene.network/api/post-convert-gem?lang=en")
            .body(body.to_string())
            .send()
            .await?;

        // let response_text = response.te.await?;
        // println!("{:?}", response_text);
        utils::format_println(name, &format!("post-convert-gem: {:?}", response.status()));
        let txt = response.text().await?;
        utils::format_println(name, &format!("post-convert-gem-result: {:?}", txt));
        Ok(())
    }

    async fn get_mining_time(&self) -> Result<i64, Box<dyn std::error::Error + Send + Sync>> {
        let client = self.request_with_token();
        let name = self.name.as_ref().unwrap();

        let response = client
            .get("https://miniapp.athene.network/api/get-mining")
            .send()
            .await?;

        let status = response.status();
        utils::format_println(name, &format!("get_mining: {:?}", status));
        if status == StatusCode::OK {
            let val: serde_json::Value = serde_json::from_str(&response.text().await?).unwrap();
            return Ok(val["data"]["remainTimeNextClaim"].as_i64().unwrap());
        }

        utils::format_error(name, "get_mining_time_error");
        Err(Box::new(AthenaErr::GetMining))
    }

    async fn post_claim_gem(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let name = self.name.as_ref().unwrap();
        let rest_mining_time = self.get_mining_time().await?;
        utils::format_println(name, &format!("get_mining_time: {}", rest_mining_time));

        if rest_mining_time <= 0i64 {
            sleep(Duration::from_secs(1)).await;

            let client = self.request_with_token();

            let response = client
                .post("https://miniapp.athene.network/api/post-claim-gem?lang=en")
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

    async fn post_convert_gem(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let name = self.name.as_ref().unwrap();
        let tap_data = self.get_tap_earn().await?;
        let total_tap = ((utils::get_current_timestamp() - tap_data.number_tap) / 100) - 100;

        utils::format_println(name, &format!("now tap count: {}", total_tap));
        if total_tap >= tap_data.min_ec as i64 {
            sleep(Duration::from_secs(1)).await;

            let txt = concat_str(tap_data.number_tap, total_tap);
            utils::format_println(
                name,
                &format!("{}, gold exchange: {}", utils::now(), total_tap),
            );
            let re = utils::rsa_encrypt(&txt);
            self.post_conver_gem(re).await?;
        }

        Ok(())
    }

    async fn post_mystery_box_claim(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let client = self.request_with_token();
        let name = self.name.as_ref().unwrap();

        let response = client
            .post("https://miniapp.athene.network/api/post-mystery-box-claim/?lang=en")
            .body("{}")
            .send()
            .await?;

        utils::format_println(
            name,
            &format!("post_mystery_box_claim_status: {:?}", response.status()),
        );
        Ok(())
    }

    async fn post_premium_pick(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let client = self.request_with_token();
        let name = self.name.as_ref().unwrap();

        let body = json!({
            "packageName": "Bronze", // TODO: select package
        });
        let response = client
            .post("https://miniapp.athene.network/api/post-premium-pick/?lang=en")
            .body(body.to_string())
            .send()
            .await?;

        utils::format_println(
            name,
            &format!("post_premium_pick_status: {:?}", response.status()),
        );
        Ok(())
    }

    async fn post_quest_reward(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let client = self.request_with_token();
        let name = self.name.as_ref().unwrap();

        let body = json!({
            "quest": 8, // daily quest
        });
        let response = client
            .post("https://miniapp.athene.network/api/post-quest-reward/?lang=en")
            .body(body.to_string())
            .send()
            .await?;

        utils::format_println(
            name,
            &format!("post_quest_reward_status: {:?}", response.status()),
        );
        Ok(())
    }

    async fn schedule1(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        loop {
            log::info!("schedule1 start");
            let _ = futures::join!(
                self.post_check_in(),
                self.post_mystery_box_claim(),
                self.post_premium_pick(),
                self.post_quest_reward(),
            );
            sleep(Duration::from_secs(60 * 60 * 12)).await;
        }
    }

    async fn schedule2(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        loop {
            log::info!("schedule2 start");
            self.post_claim_gem().await?;
            sleep(Duration::from_secs(60 * 60 * 6)).await;
        }
    }

    async fn schedule3(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        loop {
            log::info!("schedule3 start");
            self.post_convert_gem().await?;
            sleep(Duration::from_secs(60 * 2)).await;
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

async fn main_loop(user: User) {
    let user = Arc::new(user);

    let _ = tokio::join!(user.schedule1(), user.schedule2(), user.schedule3());
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    colog::init();

    // read user token from file
    let file_path = std::env::current_dir().unwrap().join("user.json");
    log::info!("file_path: {:?}", file_path);
    let users = read_config_json(file_path.to_str().unwrap());
    let arc_users: Arc<tokio::sync::Mutex<HashMap<String, User>>> =
        Arc::new(tokio::sync::Mutex::new(HashMap::new()));

    let mut handles = vec![];
    for (name, mut user) in users {
        let file_path = file_path.clone();
        let clone_users = Arc::clone(&arc_users);

        handles.push(tokio::spawn(async move {
            if user.access_token.is_none() {
                user.login().await.unwrap();
            }
            {
                let mut mutex = clone_users.lock().await;
                mutex.insert(name.clone(), user.clone());
                // write back to file
                write_config_json(file_path.to_str().unwrap(), &mutex);
            }
            user.name = Some(name.clone());
            main_loop(user).await;
        }));
    }

    let _ = futures::future::join_all(handles).await;

    Ok(())
}
