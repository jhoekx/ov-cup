// SPDX-FileCopyrightText: 2021 Jeroen Hoekx
// SPDX-License-Identifier: AGPL-3.0-or-later

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ArgumentsError {
    #[error("Invalid cup, valid cups are: city-cup, forest-cup, kampioen")]
    UnknownCup,
}

pub fn parse_cup(flag: &str) -> Result<String, ArgumentsError> {
    if flag == "city-cup" || flag == "forest-cup" || flag == "kampioen" {
        Ok(flag.to_owned())
    } else {
        Err(ArgumentsError::UnknownCup)
    }
}
