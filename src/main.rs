use rusqlite::params;
use serde_json::Value;
use std::alloc::System;
use std::fs::File;
use std::sync::mpsc;
use std::thread;
use std::time::SystemTime;
use time;
use timer;
use tokio::runtime;
use yaml_rust::YamlLoader;

mod screqs;
use crate::screqs::requests::{SearchMetadata, TwitterToken};
use screqs::requests as reqs;
use screqs::storage;
use std::error::Error;
use std::io::Read;

struct AppConfig {
    consumer_key: String,
    consumer_secret: String,
    app_token: String,
    app_secret: String,
}

fn load_config(config_path: &str) -> Result<AppConfig, dyn Error> {
    let mut config_file = File::open(config_path)?;
    let mut content = String::new();
    let sz = config_file.read_to_string(&mut content)?;
    let docs = YamlLoader::load_from_str(&content)?;
    let doc = &docs[0];

    let config = AppConfig {
        consumer_key: String::from(doc["consumer_key"].as_str()),
        consumer_secret: String::from(doc["consumer_secret"].as_str()),
        app_token: String::from(doc["app_token"].as_str()),
        app_secret: String::from(doc["app_secret"].as_str()),
    };

    return Ok(config);
}

fn twitter_api_test() {
    let config = load_config("resources/app_config.yaml").unwrap();

    let mut tokio_rt = tokio::runtime::Runtime::new().unwrap();

    let creds = reqs::create_bearer_token_creds(&config.consumer_key, &config.consumer_secret);
    let token_future = reqs::request_bearer_token(creds.as_str());
    let token = tokio_rt.block_on(token_future).unwrap();

    let search_future = reqs::search_request(&token, "Boobs -filter:retweets");
    let search_result = tokio_rt.block_on(search_future).unwrap();

    let search_metadata = reqs::get_search_metadata_from_json(&search_result).unwrap();
    let tweets_raw = reqs::get_tweets_from_json(&search_result).unwrap();
    let tweets = reqs::get_tweet_infos_from_tweets(tweets_raw).unwrap();

    for tweet in tweets {
        println!("{:?}", tweet);
    }

    println!("{:?}", search_metadata);
}

fn twitter_api_pagination_test() {
    let config = load_config("resources/app_config.yaml").unwrap();

    let mut tokio_rt = tokio::runtime::Runtime::new().unwrap();
    let conn = screqs::storage::create_conn().unwrap();

    let creds = reqs::create_bearer_token_creds(&config.consumer_key, &config.consumer_secret);
    let token_future = reqs::request_bearer_token(creds.as_str());
    let token = tokio_rt.block_on(token_future).unwrap();

    let search_future = reqs::search_request(&token, "Boobs -filter:retweets");
    let search_result = tokio_rt.block_on(search_future).unwrap();

    let extract_meta_and_tweets =
        |search_json: &reqs::SearchJSON| -> (SearchMetadata, Vec<reqs::TweetInfo>) {
            let search_metadata = reqs::get_search_metadata_from_json(search_json).unwrap();
            let tweets_raw = reqs::get_tweets_from_json(search_json).unwrap();
            let tweets = reqs::get_tweet_infos_from_tweets(tweets_raw).unwrap();
            return (search_metadata, tweets);
        };

    let (search_metadata, tweets) = extract_meta_and_tweets(&search_result);

    screqs::storage::create_tweet_table(&conn).unwrap();
    screqs::storage::insert_replace_tweets(&conn, &tweets).unwrap();

    for tweet in tweets {
        println!("{:?}", tweet);
    }

    println!("{:?}", search_metadata);

    let timer = timer::Timer::new();
    let duration = time::Duration::milliseconds(2200);

    let (tx, rx) = mpsc::channel();
    tx.send((token, search_metadata)).unwrap();

    let (end_tx, end_rx) = mpsc::channel::<()>();

    let func = move || -> () {
        let (token, meta) = rx.recv().unwrap();

        let search_fut = reqs::search_request_next(&token, &meta);
        let search_res = match tokio_rt.block_on(search_fut) {
            Ok(v) => v,
            Err(reqs::Error::NoNextResult()) => {
                end_tx.send(());
                return;
            }
            Err(e) => panic!("{:?}", e),
        };

        let (next_metadata, tweets) = extract_meta_and_tweets(&search_res);

        screqs::storage::insert_replace_tweets(&conn, &tweets).unwrap();

        for tweet in tweets {
            println!("{:?}", tweet);
        }

        println!("{:?}", next_metadata);

        tx.send((token.clone(), next_metadata)).unwrap();
    };

    let _guard = timer.schedule_repeating(duration, func);

    match end_rx.recv() {
        Ok(_) => println!("End of results reached."),
        Err(e) => panic!("{:?}", e),
    }
}

fn sqlite_test() -> Result<(), String> {
    let conn_res = screqs::storage::create_conn();
    let conn = conn_res.unwrap();

    return storage::create_tweet_table(&conn);
}

fn fixed_interval_test() -> Result<(), String> {
    let timer = timer::Timer::new();
    let duration = time::Duration::milliseconds(1000);

    let mut time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let start = time;

    let func = move || -> () {
        let prev = time;
        time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis();

        print!(
            "ms since start: {}\nms this tick: {}\n",
            time - start,
            time - prev
        );
    };

    let guard = timer.schedule_repeating(duration, func);

    thread::sleep(time::Duration::seconds(30).to_std().unwrap());

    return Ok(());
}

fn main() {
    //sqlite_test();
    //fixed_interval_test();
    //twitter_api_test();
    twitter_api_pagination_test();
}
