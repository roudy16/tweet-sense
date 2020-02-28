use base64;
use rand::prelude::*;
use reqwest::Client;
use serde_json::{from_str, json, Map, Value};
use urlencoding;

// twitter access
// token: 703880420-JgOPobTDO6U7aPJaFdwBImxTowBhiTXTZiBT3V12
// secret: 1F8XJN65rF1OfKYu0K4qUv71wSIDk1DX2xpzsuKJYKtis

#[derive(Debug)]
pub struct TweetInfo {
    user_id: i64,
    user_name: String,
    user_screen_name: String,
    tweet_id: i64,
    tweet_text: String,
    truncated: bool,
}

impl TweetInfo {
    pub fn user_id(&self) -> i64 {
        return self.user_id;
    }

    pub fn user_name(&self) -> &String {
        return &self.user_name;
    }

    pub fn user_screen_name(&self) -> &String {
        return &self.user_screen_name;
    }

    pub fn tweet_id(&self) -> i64 {
        return self.tweet_id;
    }

    pub fn tweet_text(&self) -> &String {
        return &self.tweet_text;
    }

    pub fn truncated(&self) -> bool {
        return self.truncated;
    }
}

#[derive(Debug)]
pub struct SearchMetadata {
    completed_in: f64,
    max_id: i64,
    next_results: Option<String>,
}

#[derive(Clone, Debug)]
pub struct TwitterToken {
    token: String,
}

#[derive(Debug)]
pub struct SearchJSON {
    val: Value,
}

#[derive(Debug)]
pub struct AuthJSON {
    val: Value,
}

#[derive(Debug)]
pub enum ApiJSON {
    Auth(AuthJSON),
    Search(SearchJSON),
}

#[derive(Debug)]
pub enum Error {
    Reqwest(reqwest::Error),
    Serde(serde_json::Error),
    String(String),
    NoNextResult(),
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        return Error::Reqwest(e);
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        return Error::Serde(e);
    }
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        return Error::String(s);
    }
}

pub fn create_bearer_token_creds(key: &str, secret: &str) -> String {
    let combined = format!("{}:{}", key, secret);
    let b64 = base64::encode(&combined);
    return b64;
}

fn get_access_token_from_json(auth: &AuthJSON) -> Result<TwitterToken, serde_json::Error> {
    let token = auth.val["access_token"].as_str().unwrap().to_string();
    return Ok(TwitterToken { token });
}

pub async fn request_bearer_token(creds: &str) -> Result<TwitterToken, Error> {
    let client = reqwest::Client::new();
    let params = [("grant_type", "client_credentials")];
    let content = client
        .post("https://api.twitter.com/oauth2/token")
        .header("Authorization", format!("Basic {}", creds))
        .header(
            "Content-Type",
            "application/x-www-form-urlencoded;charset=UTF-8",
        )
        .form(&params)
        .send()
        .await?
        .text()
        .await?;

    let auth_val: serde_json::Value = serde_json::from_str(content.as_str())?;
    let auth = AuthJSON { val: auth_val };

    let token_result = get_access_token_from_json(&auth);
    let token = match token_result {
        Ok(res) => res,
        _ => return Err(Error::String("Fail".to_string())),
    };

    return Ok(token);
}

fn get_tweet_json_from_response(response: &str) -> Result<SearchJSON, Error> {
    let resp_json: Value = match from_str(response) {
        Ok(val) => val,
        Err(e) => return Err(Error::from(e)),
    };
    return Ok(SearchJSON { val: resp_json });
}

pub fn get_tweets_from_json(search_json: &SearchJSON) -> Result<&Vec<Value>, Error> {
    let status_res = &search_json.val["statuses"];

    let tweets = match status_res {
        Value::Array(v) => Ok(v),
        _ => Err(Error::from("Expect Array for 'statuses'".to_string())),
    };

    return tweets;
}

pub fn get_search_metadata_from_json(search_json: &SearchJSON) -> Result<SearchMetadata, Error> {
    let status_res = &search_json.val["search_metadata"];

    let metadata_raw = match status_res {
        Value::Object(v) => v,
        _ => {
            return Err(Error::from(
                "Expect Object for 'search_metadata'".to_string(),
            ))
        }
    };
    let has_next = metadata_raw.contains_key("next_results");

    let metadata = SearchMetadata {
        completed_in: metadata_raw["completed_in"].as_f64().unwrap(),
        max_id: metadata_raw["max_id"].as_i64().unwrap(),
        next_results: match has_next {
            true => Some(metadata_raw["next_results"].as_str().unwrap().to_string()),
            false => None,
        },
    };

    return Ok(metadata);
}

pub fn get_tweet_info_from_tweet(tweet: &Value) -> Result<TweetInfo, Error> {
    let info = TweetInfo {
        user_id: tweet["user"]["id"].as_i64().unwrap(),
        user_name: tweet["user"]["name"].as_str().unwrap().to_string(),
        user_screen_name: tweet["user"]["screen_name"].as_str().unwrap().to_string(),
        tweet_id: tweet["id"].as_i64().unwrap(),
        tweet_text: tweet["text"].as_str().unwrap().to_string(),
        truncated: tweet["truncated"].as_bool().unwrap(),
    };

    return Ok(info);
}

pub fn get_tweet_infos_from_tweets(tweets: &Vec<Value>) -> Result<Vec<TweetInfo>, Error> {
    let res: Vec<TweetInfo> = tweets
        .into_iter()
        .map(|t| get_tweet_info_from_tweet(t).unwrap())
        .collect();

    return Ok(res);
}

fn build_query_string(params: &[(&str, &str)]) -> String {
    let mut sb = String::with_capacity(32);
    sb.push('?');
    for (i, param) in params.iter().enumerate() {
        if i != 0 {
            sb.push('&');
        }
        sb.push_str(urlencoding::encode(param.0).as_str());
        sb.push('=');
        sb.push_str(urlencoding::encode(param.1).as_str());
    }
    return sb;
}

fn build_query_from_next_results(query_in: &str) -> String {
    return format!("{}{}", query_in, "&include_entities=0");
}

async fn search_request_helper(token: &TwitterToken, url_query: &str) -> Result<String, Error> {
    let endpoint = "https://api.twitter.com/1.1/search/tweets.json";
    let req_url = format!("{}{}", endpoint, url_query);

    let client = reqwest::Client::new();
    let content = client
        .get(&req_url)
        .bearer_auth(&token.token)
        .send()
        .await?
        .text()
        .await?;

    return Ok(content);
}

pub async fn search_request_next(
    token: &TwitterToken,
    meta: &SearchMetadata,
) -> Result<SearchJSON, Error> {
    if meta.next_results.is_none() {
        return Err(Error::NoNextResult());
    }

    let q = build_query_from_next_results(meta.next_results.as_ref().unwrap());
    let content = search_request_helper(token, &q).await?;
    return get_tweet_json_from_response(content.as_str());
}

pub async fn search_request(token: &TwitterToken, query: &str) -> Result<SearchJSON, Error> {
    let params = [
        ("q", query),
        ("count", "100".as_ref()),
        ("include_entities", "0".as_ref()),
    ];

    let q = build_query_string(&params);
    let content = search_request_helper(token, &q).await?;
    return get_tweet_json_from_response(content.as_str());
}

// Chebs, hooters, boobs, fun bags, tits, hoo hars, breasts, baps, rack, mammaries, melons, puppie, tiddies

// Track oldest key
