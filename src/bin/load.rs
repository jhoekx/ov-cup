// SPDX-FileCopyrightText: 2021 Jeroen Hoekx
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use ov_cup::db::LocalDatabase;
use ov_cup::AgeClassOverride;
use structopt::StructOpt;

use ov_cup::cli;
use ov_cup::webres;

#[derive(StructOpt, Debug)]
#[structopt(name = "load")]
struct Opt {
    #[structopt(long, default_value = "forest-cup", parse(try_from_str = cli::parse_cup))]
    cup: String,

    #[structopt(long)]
    season: String,

    #[structopt(name = "FILE")]
    paths: Vec<String>,

    #[structopt(long)]
    by_class: Option<bool>,

    #[structopt(long, default_value = "overrides.json")]
    overrides: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();

    let cup = opt.cup.to_owned();
    let season = opt.season.to_owned();
    let overrides = read_overrides_json(opt.overrides)?
        .into_iter()
        .filter(|age_class_override| {
            age_class_override.cup == cup && age_class_override.season == season
        })
        .collect();
    let options = ov_cup::ResultProcessingOptions {
        cup,
        season,
        results_by_class: opt.by_class,
        overrides,
    };

    let db_path = PathBuf::from("ov.sqlite");
    let db = LocalDatabase::new(db_path);
    ov_cup::create_database(&db)?;

    for path in opt.paths {
        let event = webres::read_event_json(path)?;
        ov_cup::store_event(&db, event, &options)?;
    }

    Ok(())
}

pub fn read_overrides_json(
    path: String,
) -> Result<Vec<AgeClassOverride>, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let overrides = serde_json::from_reader(reader)?;
    Ok(overrides)
}
