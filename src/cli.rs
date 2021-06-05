use thiserror::Error;

#[derive(Error, Debug)]
pub enum ArgumentsError {
    #[error("Invalid cup, valid cups are: city-cup, forest-cup")]
    UnknownCup,
}

pub fn parse_cup(flag: &str) -> Result<String, ArgumentsError> {
    if flag == "city-cup" || flag == "forest-cup" {
        Ok(flag.to_owned())
    } else {
        Err(ArgumentsError::UnknownCup)
    }
}