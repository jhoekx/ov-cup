use std::collections::HashMap;

use chrono::{NaiveTime, Timelike};
use rusqlite::{params, Connection};
use itertools::Itertools;

#[derive(Debug)]
struct Performance {
    name: String,
    club: String,
    age_class: String,
    category_name: String,
    time: NaiveTime,
    score: u32,
}

fn total_seconds(time: impl Timelike) -> u32 {
    time.second() + time.minute() * 60 + time.hour() * 60 * 60
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::open("ov.sqlite")?;
    let age_class = "H35";

    // Find all results of all runners with at least one ranking in the given category
    // (TODO in events in the given season).
    let mut stmt = conn.prepare(
        "
        select Runner.name, Runner.club, Result.age_class, Result.category_name, Result.time
        from Result join Runner on Result.runner_id = Runner.id
        where Runner.id in (
            select Runner.id
            from Runner join Result on Runner.id = Result.Runner_id
            where Result.age_class = ?
        )
    ",
    )?;
    let results: Vec<Performance> = stmt
        .query_map(params![age_class], |row| {
            Ok(Performance {
                name: row.get(0)?,
                club: row.get(1)?,
                age_class: row.get(2)?,
                category_name: row.get(3)?,
                time: row.get(4)?,
                score: 0,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    // TODO: Keep only courses that are valid for the age class of the result

    // TODO: Keep only results of runners where the last age class equals the given age class
    //       This filters out runners who moved to a different category,
    //       while keeping the runners that moved into this category.

    // Find the best results in all courses that someone of the given age class participated in
    // TODO: course = event, category_name
    let courses: Vec<String> = results.iter().map(|result| result.category_name.to_string()).unique().collect();
    let mut stmt = conn.prepare("
        select Result.time
        from Result
        where Result.category_name = ?
        order by Result.time asc
        limit 1
    ")?;
    let mut fastest_times = HashMap::new();
    for course in courses {
        let fastest_time: NaiveTime = stmt.query_row(params![course], |row| {
            Ok(row.get(0)?)
        })?;
        fastest_times.insert(course, total_seconds(fastest_time));
    }

    for mut result in results {
        result.score = 1000 * fastest_times.get(&result.category_name).unwrap() / total_seconds(result.time);
        dbg!(result);
    }


    // create intermediate rankings to show result evolution

    Ok(())
}
