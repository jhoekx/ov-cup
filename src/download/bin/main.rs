use std::{collections::HashMap, fmt::Display, str::FromStr};

use chrono::{DateTime, NaiveTime, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Deserializer};

#[derive(Debug, Deserialize)]
struct CourseResult {
    name: String,
    club: String,
    #[serde(rename = "ageclass")]
    age_class: String,
    #[serde(deserialize_with = "from_str")]
    position: u32,
    time: NaiveTime,
    status: String,
}

#[derive(Debug, Deserialize)]
struct Category {
    name: String,
    #[serde(deserialize_with = "from_str")]
    distance: u32,
    #[serde(deserialize_with = "from_str")]
    climb: u32,
    results: Vec<CourseResult>,
}

#[derive(Debug, Deserialize)]
struct Event {
    date: DateTime<Utc>,
    name: String,
    location: String,
    categories: HashMap<String, Category>,
}

fn from_str<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: FromStr,
    T::Err: Display,
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    T::from_str(&s).map_err(serde::de::Error::custom)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_id = 2845;
    let event_url = url::Url::parse_with_params(
        "http://helga-o.com/webres/ws.php",
        &[("lauf", event_id.to_string())],
    )?;
    let event: Event = reqwest::blocking::get(event_url)?.json()?;

    let conn = Connection::open("ov.sqlite")?;
    conn.pragma_update(None, "foreign_keys", &"on")?;
    conn.pragma_update(None, "journal_mode", &"WAL")?;
    conn.execute_batch(
        "
        create table if not exists Runner (
            id integer primary key autoincrement,
            name text not null,
            club text not null,

            unique(name)
        );

        create table if not exists Event (
            id integer primary key autoincrement,
            name text not null,
            location text not null,
            date text not null,

            unique(name, date)
        );

        create table if not exists Result (
            id integer primary key autoincrement,
            event_id integer not null,
            runner_id integer not null,
            category_name text not null,
            age_class text not null,
            position integer not null,
            time text not null,

            foreign key(event_id) references Event(id),
            foreign key(runner_id) references Runner(id)
        )
    ",
    )?;

    conn.execute(
        "
        insert into Event (name, location, date) values (?, ?, ?)
        on conflict (name, date) do update set location = excluded.location;
    ",
        params![event.name, event.location, event.date],
    )?;
    let event_db_id: i64 = conn.query_row(
        "
        select id from Event where name = ? and date = ?
    ",
        params![event.name, event.date],
        |row| row.get(0),
    )?;
    conn.execute(
        "
        delete from Result where event_id = ?
    ",
        params![event_db_id],
    )?;

    for (_, category) in &event.categories {
        for result in &category.results {
            if result.status != "OK" || result.position == 0 {
                continue;
            }

            conn.execute(
                "
                insert into Runner (name, club) values (?, ?)
                on conflict (name) do update set club = excluded.club;
            ",
                params![result.name, result.club],
            )?;
            let runner_db_id: i64 = conn.query_row(
                "
                select id from Runner where name = ?
            ",
                params![result.name],
                |row| row.get(0),
            )?;

            conn.execute(
                "
                insert into Result (event_id, runner_id, category_name, age_class, position, time)
                values (?, ?, ?, ?, ?, ?)
            ",
                params![
                    event_db_id,
                    runner_db_id,
                    category.name,
                    result.age_class,
                    result.position,
                    result.time
                ],
            )?;
        }
    }
    Ok(())
}
