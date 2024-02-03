// SPDX-FileCopyrightText: 2021 Jeroen Hoekx
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;

use clap::Parser;
use ov_cup::db::LocalDatabase;

use ov_cup::calculate_ranking;
use ov_cup::cli;

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, default_value = "forest-cup", value_parser = cli::parse_cup)]
    cup: String,

    #[arg(long, default_value = "2020")]
    season: i16,

    #[arg(long, default_value = "H35")]
    age_class: String,

    #[arg(long, default_value = "4")]
    events_count: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let db = LocalDatabase::new(PathBuf::from("ov.sqlite"));
    let ranking = calculate_ranking(
        &db,
        args.cup,
        args.season,
        args.age_class,
        args.events_count,
    )?;
    dbg!(ranking);
    Ok(())
}
