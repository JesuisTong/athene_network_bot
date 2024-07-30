use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use chrono::Local;
use std::time::{SystemTime, UNIX_EPOCH};

use rsa::Oaep;
use rsa::{pkcs8::DecodePublicKey, RsaPublicKey};
use sha2::Sha256;

use reqwest::header::{
    HeaderMap, HeaderValue, ACCEPT, ACCEPT_LANGUAGE, CACHE_CONTROL, CONTENT_TYPE, PRAGMA, REFERER,
    REFERRER_POLICY, USER_AGENT,
};

// 加密算法
pub fn rsa_encrypt(t: &str) -> String {
    let key = "-----BEGIN PUBLIC KEY-----
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAwmkourU0WsNzc0mcb6a7
xRBgN4FaA7ak/82zeIMBQf0/uTY42p3uW5IgSx56DpnuGnrFAYLDDDu96lhRNpNK
r6uaIBuhX7tg+4m26KDAEDHyTWgXEsZkClTeAl15gP0NoKXYGCr+rGBe2uvjFVJU
ML+J9kPIVnJ99jxJ8EWiRJ9L/qhr5C04Z1QhNnF3fiaaeXlGgQLwwbSIKnc3ypuw
E/0zrAGJ+1WiidCbQqqclGglpcSFWtF8znte+H/jlk1+0rypil4AkNPdN+mbAb8w
HkQhWA50hXFxmCnIJjw10YfZAFkMzaFKNVZpOmnTvZrmlQShsOLUci8q45kN1jsU
pQIDAQAB
-----END PUBLIC KEY-----";

    let public_key = RsaPublicKey::from_public_key_pem(&key).unwrap();

    let mut rng = rand::thread_rng();
    let padding = Oaep::new::<Sha256>();
    let encrypted_data = public_key.encrypt(&mut rng, padding, t.as_bytes()).unwrap();
    STANDARD.encode(&encrypted_data)
}

pub fn now() -> String {
    Local::now().format("%F %T").to_string()
}

pub fn get_current_timestamp() -> i64 {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");

    since_the_epoch.as_millis() as i64
}

pub fn format_println(name: &str, msg: &str) {
    println!("[{}] [{}]: {}", now(), name, msg);
}

pub fn init_headers(h: &mut HeaderMap) -> &mut HeaderMap {
    h.insert(ACCEPT, HeaderValue::from_static("*/*"));
    h.insert(
        ACCEPT_LANGUAGE,
        HeaderValue::from_static("zh-CN,zh;q=0.9,en;q=0.8,en-GB;q=0.7,en-US;q=0.6"),
    );
    h.insert(CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    h.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    h.insert(PRAGMA, HeaderValue::from_static("no-cache"));
    h.insert(
        REFERER,
        HeaderValue::from_static("https://miniapp.athene.network/mining/"),
    );
    h.insert("priority", HeaderValue::from_static("u=1, i"));
    h.insert("sec-ch-ua", HeaderValue::from_static("\"\""));
    h.insert("sec-ch-ua-mobile", HeaderValue::from_static("?1"));
    h.insert("sec-ch-ua-platform", HeaderValue::from_static("\"\""));
    h.insert("sec-fetch-dest", HeaderValue::from_static("empty"));
    h.insert("sec-fetch-mode", HeaderValue::from_static("cors"));
    h.insert("sec-fetch-site", HeaderValue::from_static("same-origin"));
    h.insert(
        REFERER,
        HeaderValue::from_static("https://miniapp.athene.network/mining/"),
    );
    h.insert(
        REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    h.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (iPhone; CPU iPhone OS 16_6 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.6 Mobile/15E148 Safari/604.1 Edg/126.0.0.0"));

    h
}
