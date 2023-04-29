// Database connections

// SPDX-FileCopyrightText: 2023 Jeroen Hoekx
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;

use rusqlite::Connection;

pub trait Database {
    fn open(&self) -> anyhow::Result<Connection>;
}

pub struct LocalDatabase {
    path: PathBuf,
}

impl LocalDatabase {
    pub fn new(path: PathBuf) -> Self {
        LocalDatabase { path }
    }
}

impl Database for LocalDatabase {
    fn open(&self) -> anyhow::Result<Connection> {
        Ok(Connection::open(&self.path)?)
    }
}
