use crate::screqs::requests::TweetInfo;
use rusqlite::{params, Connection, Result, ToSql};
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

fn create_prep_value_packs(num_packs: usize, num_params_per_pack: u32) -> String {
    let mut sb = String::with_capacity(256);

    for j in 0..num_packs {
        sb.push('(');

        for i in 1..(num_params_per_pack) {
            sb.push('?');
            sb.push_str(&i.to_string());
            sb.push(',');
        }

        sb.push_str(&num_params_per_pack.to_string());
        sb.push(')');

        if j != (num_packs - 1) {
            sb.push(',');
        }
    }

    return sb;
}

pub fn insert_replace_tweets(
    conn: &Connection,
    twinfos: &Vec<TweetInfo>,
) -> std::result::Result<(), String> {
    let num_fields = 6;

    // TODO: Batches
    //let packs = create_prep_value_packs(twinfos.len(), num_fields);

    let q = "insert or replace into mam_tweets
        (tweet_id, user_id, truncated, tweet_text, user_name, user_screen_name) VALUES
        (?1,?2,?3,?4,?5,?6);";

    println!("{}", q);

    for twinfo in twinfos {
        let params = params!(
            twinfo.tweet_id().to_string(),
            twinfo.user_id().to_string(),
            if twinfo.truncated() {
                "1".to_string()
            } else {
                "0".to_string()
            },
            twinfo.tweet_text().to_string(),
            twinfo.user_name().to_string(),
            twinfo.user_screen_name().to_string(),
        );

        conn.execute(&q, params).unwrap();
    }

    return Ok(());
}
