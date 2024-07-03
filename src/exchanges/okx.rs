use anyhow::{anyhow, Error};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::Utc;
use hmac::{Hmac, Mac};
use reqwest::header::HeaderMap;
use reqwest::StatusCode;
use serde_json::{to_string, Value};
use sha2::Sha256;
use std::collections::HashMap;
use std::string::String;
use crate::constants::{Side};
use url::form_urlencoded;

use crate::exchanges::base::{BaseExchange, RestClient};
use crate::exchanges::exchange_types::FetchPositionParams;

pub struct OkxExchange {
    key: String,
    secret: String,
    passphrase: String,
    base_url: String,
    is_demo: bool,
    rest_client: RestClient,
}

impl OkxExchange {
    pub fn new(configs: &HashMap<String, String>) -> Self {
        let key: String = configs.get("key").unwrap().to_string();
        let secret: String = configs.get("secret").unwrap().to_string();
        let passphrase:String = configs.get("passphrase").unwrap().to_string();
        let base_url: String = "https://www.okx.com".to_string();
        let is_demo: bool = configs.get("is_demo").and_then(|s| s.parse::<bool>().ok()).unwrap_or(false);
        let rest_client: RestClient = RestClient::new();

        OkxExchange {
            key,
            secret,
            passphrase,
            base_url,
            is_demo,
            rest_client,
        }
    }

    // Signature definition is specific to the exchange
    fn generate_signature(
        &self,
        method: &str,
        url: &str,
        query_string: &str,
        body: &str,
    ) -> (String, String) {
        let timestamp = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let pre_hash = format!("{}{}{}{}{}", timestamp, method, url, query_string, body);
        let mut mac = Hmac::<Sha256>::new_from_slice(self.secret.as_bytes()).unwrap();
        mac.update(pre_hash.as_bytes());
        let signature = mac.finalize().into_bytes();
        let encoded_signature = STANDARD.encode(&signature);
        (encoded_signature, timestamp)
    }

    // Headers definition are specific to the exchange
    fn get_headers(&self, signature: &str, timestamp: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", "application/json".parse().unwrap());
        headers.insert("OK-ACCESS-KEY", self.key.parse().unwrap());
        headers.insert("OK-ACCESS-SIGN", signature.parse().unwrap());
        headers.insert("OK-ACCESS-TIMESTAMP", timestamp.parse().unwrap());
        headers.insert("OK-ACCESS-PASSPHRASE", self.passphrase.parse().unwrap());
        if self.is_demo {
            headers.insert("x-simulated-trading", "1".parse().unwrap());
        }

        headers
    }

    async fn send_request(
        &self,
        method: &str,
        endpoint: &str,
        body: Option<HashMap<String, String>>,
    ) -> Result<HashMap<String, String>, Error> {
        let query_string = match &body {
            Some(map) => {
                let query = form_urlencoded::Serializer::new(String::new())
                    .extend_pairs(map)
                    .finish();
                format!("?{}", query)
            }
            None => String::new(),
        };

        let body_string = match method {
            "POST" => match &body {
                Some(map) => to_string(map).unwrap_or_else(|_| String::new()),
                None => String::new(),
            },
            _ => String::new(),
        };

        let (encoded_signature, timestamp) =
            self.generate_signature(method, endpoint, &query_string, &body_string);

        let headers: HeaderMap = self.get_headers(&encoded_signature, &timestamp);
        let url = format!("{}{}{}", self.base_url, endpoint, query_string);
        let response = self
            .rest_client
            .send_request(method, &url, Some(headers), body)
            .await?;

        let status: StatusCode = response.status();
        let text = response.text().await?;
        if status.is_success() {
            let result: HashMap<String, String> = serde_json::from_str(&text)?;
            Ok(result)
        } else {
            Err(anyhow!(
                "{} request failed with status: {} and body: {}",
                method,
                status,
                text
            ))
        }
    }
}

impl BaseExchange for OkxExchange {
    async fn get_ticker(&self, symbol: &str) -> Result<HashMap<String, String>, Error> {
        todo!()
    }

    async fn fetch_positions(
        &self,
        params: FetchPositionParams,
    ) -> Result<Vec<HashMap<String, String>>, Error> {
        todo!()
    }

    async fn fetch_balances(&self) -> Result<Vec<HashMap<String, String>>, Error> {
        let endpoint: &str = "/api/v5/account/balance";
        let response = self.send_request("GET", endpoint, None).await?;

        let response_value: Value = serde_json::to_value(response)?;

        if let Some(data) = response_value.get("data") {
            if data.is_array() {
                let balances: Vec<HashMap<String, String>> = serde_json::from_value(data.clone())?;
                Ok(balances)
            } else {
                Err(anyhow!("Data is not an array."))
            }
        } else {
            Err(anyhow!("No balance data found in the response."))
        }
    }

    async fn get_bbo_price(&self, symbol: &str, side: Side) -> Result<f64, Error> {
        todo!()
    }
}
