// SPDX-FileCopyrightText: 2021 Jeroen Hoekx
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::Path;

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
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();
    let db_path = Path::new("ov.sqlite");

    ov_cup::create_database(db_path)?;
    for path in opt.paths {
        let event = webres::read_event_json(path)?;
        let options = ov_cup::ResultProcessingOptions {
            cup: opt.cup.to_owned(),
            season: opt.season.to_owned(),
            results_by_class: opt.by_class,
        };
        ov_cup::store_event(db_path, event, options)?;
    }

    Ok(())
}
