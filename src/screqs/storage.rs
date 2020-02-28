use crate::screqs::requests::TweetInfo;
use rusqlite::{params, Connection, Result};
use std::path::Path;

pub fn create_conn() -> Result<Connection> {
    let path = Path::new("ts.db");
    let conn = Connection::open(path);
    return conn;
}

pub fn create_tweet_table(conn: &Connection) -> std::result::Result<(), String> {
    conn.execute(
        "create table if not exists mam_tweets
        (
        tweet_id int not null
          constraint mam_tweets_pk
          primary key,
        user_id int not null,
        truncated int not null,
        tweet_text text not null,
        user_name text not null,
        user_screen_name text not null
        );",
        params![],
    )
    .unwrap();

    conn.execute(
        "create index if not exists mam_tweets_user_id_index
        on mam_tweets (user_id);",
        params![],
    )
    .unwrap();

    conn.execute(
        "create index if not exists mam_tweets_truncated_index
        on mam_tweets (truncated);",
        params![],
    )
    .unwrap();

    return Ok(());
}

fn create_tweet_value_pack(twinfo: &TweetInfo) -> String {
    return format!(
        "({},{},{},'{}','{}','{}')",
        twinfo.tweet_id(),
        twinfo.user_id(),
        twinfo.truncated() as u8,
        twinfo.tweet_text(),
        twinfo.user_name(),
        twinfo.user_screen_name(),
    );
}

fn create_tweet_value_packs(twinfos: &Vec<TweetInfo>) -> String {
    let packs: Vec<String> = twinfos
        .into_iter()
        .map(|twinfo| create_tweet_value_pack(twinfo))
        .collect();

    return packs.join(",");
}

pub fn insert_replace_tweets(
    conn: &Connection,
    twinfos: &Vec<TweetInfo>,
) -> std::result::Result<(), String> {
    let packs = create_tweet_value_packs(twinfos);
    let q = format!(
        "insert or replace into mam_tweets
        (tweet_id, user_id, truncated, tweet_text, user_name, user_screen_name) VALUES {};",
        packs
    );

    println!("{}", q);

    conn.execute(&q, params![]).unwrap();

    return Ok(());
}
