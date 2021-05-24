use std::collections::HashMap;

use chrono::{DateTime, NaiveTime, Timelike, Utc};
use itertools::Itertools;
use rusqlite::{params, Connection};

pub mod cli;
pub mod webres;

pub fn create_database() -> Result<(), Box<dyn std::error::Error>> {
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
            cup text not null,
            season text not null,
            name text not null,
            location text not null,
            date text not null,

            unique(cup, season, name, date)
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
    Ok(())
}

pub fn store_event(
    cup: String,
    season: String,
    event: webres::Event,
) -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::open("ov.sqlite")?;
    conn.execute(
        "
        insert into Event (cup, season, name, location, date) values (?, ?, ?, ?, ?)
        on conflict (cup, season, name, date) do update set location = excluded.location;
    ",
        params![cup, season, event.name, event.location, event.date],
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

    for category in event.categories.values() {
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

#[derive(Debug)]
struct Performance {
    name: String,
    club: String,
    event_id: u64,
    event_name: String,
    event_date: DateTime<Utc>,
    age_class: String,
    category_name: String,
    time: NaiveTime,
    score: u32,
}

fn total_seconds(time: impl Timelike) -> u32 {
    time.second() + time.minute() * 60 + time.hour() * 60 * 60
}

pub fn calculate_ranking(
    cup: String,
    season: String,
    age_class: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::open("ov.sqlite")?;

    // Find all results of all runners with at least one ranking in the given category
    let mut stmt = conn.prepare(
        "
        select
            Runner.name,
            Runner.club,
            Event.id,
            Event.name,
            Event.date,
            Result.age_class,
            Result.category_name,
            Result.time
        from Result join Runner on Result.runner_id = Runner.id
                    join Event on Result.event_id = Event.id
        where Event.cup = ? and Event.season = ?
          and Runner.id in (
              select Runner.id
              from Runner join Result on Runner.id = Result.Runner_id
              where Result.age_class = ?
          )
        order by Runner.name asc, Event.date asc
    ",
    )?;
    let all_results: Vec<Performance> = stmt
        .query_map(params![cup, season, age_class], |row| {
            Ok(Performance {
                name: row.get(0)?,
                club: row.get(1)?,
                event_id: row.get(2)?,
                event_name: row.get(3)?,
                event_date: row.get(4)?,
                age_class: row.get(5)?,
                category_name: row.get(6)?,
                time: row.get(7)?,
                score: 0,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    // TODO: Keep only courses that are valid for the age class of the result

    // Keep only results of runners where the last age class equals the given age class
    // This filters out runners who moved to a different category,
    // while keeping the runners that moved into this category.
    let mut results = Vec::new();
    for (_, runner_results) in &all_results
        .into_iter()
        .group_by(|result| result.name.to_owned())
    {
        let mut runner_results: Vec<Performance> = runner_results.collect();
        if runner_results.last().unwrap().age_class == age_class {
            results.append(&mut runner_results);
        }
    }

    // Find the best results in all courses that someone of the given age class participated in
    let courses: Vec<(u64, String)> = results
        .iter()
        .map(|result| (result.event_id, result.category_name.to_owned()))
        .unique()
        .collect();
    let mut stmt = conn.prepare(
        "
        select Result.time
        from Result
        where Result.event_id = ? and Result.category_name = ?
        order by Result.time asc
        limit 1
    ",
    )?;
    let mut fastest_times = HashMap::new();
    for (event_id, category_name) in courses {
        let fastest_time: NaiveTime =
            stmt.query_row(params![event_id, category_name], |row| row.get(0))?;
        fastest_times.insert((event_id, category_name), total_seconds(fastest_time));
    }

    // Calculate score for each performance based on the fastest times
    let results: Vec<Performance> = results
        .into_iter()
        .map(|result| {
            let score = 1000
                * fastest_times
                    .get(&(result.event_id, result.category_name.to_owned()))
                    .unwrap()
                / total_seconds(result.time);
            Performance { score, ..result }
        })
        .collect();

    // Calculate the total scores per runner
    for (name, runner_results) in &results
        .into_iter()
        .group_by(|result| result.name.to_owned())
    {
        let runner_results: Vec<Performance> = runner_results.collect();
        let mut scores: Vec<u32> = runner_results.iter().map(|result| result.score).collect();
        scores.sort_unstable();
        scores.reverse();
        let total_score: u32 = scores.iter().take(4).sum();
        dbg!(name, total_score, scores);
    }

    Ok(())
}
