/*  medal                                                                                                            *\
 *  Copyright (C) 2020  Bundesweite Informatikwettbewerbe                                                            *
 *                                                                                                                   *
 *  This program is free software: you can redistribute it and/or modify it under the terms of the GNU Affero        *
 *  General Public License as published  by the Free Software Foundation, either version 3 of the License, or (at    *
 *  your option) any later version.                                                                                  *
 *                                                                                                                   *
 *  This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the       *
 *  implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public      *
 *  License for more details.                                                                                        *
 *                                                                                                                   *
 *  You should have received a copy of the GNU Affero General Public License along with this program.  If not, see   *
\*  <http://www.gnu.org/licenses/>.                                                                                  */

#![cfg(feature = "rusqlite")]

extern crate rusqlite;

use config;
use rusqlite::Connection;
use time;
use time::Duration;

use db_conn::{MedalConnection, MedalObject, SignupResult};
use db_objects::*;
use helpers;

fn gen_tosql_vector() -> Vec<&'static dyn rusqlite::types::ToSql> { Vec::new() }

trait Queryable {
    fn query_map_one<T, F>(&self, sql: &str, params: &[&dyn rusqlite::types::ToSql], f: F)
                           -> rusqlite::Result<Option<T>>
        where F: FnOnce(&rusqlite::Row) -> T;
    fn query_map_many<T, F>(&self, sql: &str, params: &[&dyn rusqlite::types::ToSql], f: F) -> rusqlite::Result<Vec<T>>
        where F: FnMut(&rusqlite::Row) -> T;
    fn exists(&self, sql: &str, params: &[&dyn rusqlite::types::ToSql]) -> bool;
    fn get_last_id(&self) -> Option<i32>;

    fn reconnect_concrete(config: &config::Config) -> Self;
}

impl Queryable for Connection {
    fn query_map_one<T, F>(&self, sql: &str, params: &[&dyn rusqlite::types::ToSql], f: F)
                           -> rusqlite::Result<Option<T>>
        where F: FnOnce(&rusqlite::Row) -> T {
        let mut stmt = self.prepare(sql)?;
        let mut rows = stmt.query(params)?;
        match rows.next() {
            None => Ok(None),
            Some(Err(e)) => Err(e),
            Some(Ok(row)) => Ok(Some(f(&row))),
        }
    }

    fn query_map_many<T, F>(&self, sql: &str, params: &[&dyn rusqlite::types::ToSql], f: F) -> rusqlite::Result<Vec<T>>
        where F: FnMut(&rusqlite::Row) -> T {
        let mut stmt = self.prepare(sql)?;
        let rows = stmt.query_map(params, f)?;
        Ok(rows.map(|x| x.unwrap()).collect())
    }

    fn exists(&self, sql: &str, params: &[&dyn rusqlite::types::ToSql]) -> bool {
        let mut stmt = self.prepare(sql).unwrap();
        stmt.exists(params).unwrap()
    }

    fn get_last_id(&self) -> Option<i32> { self.query_row("SELECT last_insert_rowid()", &[], |row| row.get(0)).ok() }

    fn reconnect_concrete(config: &config::Config) -> Self {
        rusqlite::Connection::open(config.database_file.clone().unwrap()).unwrap()
    }
}

impl MedalObject<Connection> for Submission {
    fn save(&mut self, conn: &Connection) {
        match self.get_id() {
            Some(_id) => unimplemented!(),
            None => {
                let query = "INSERT INTO submission (task, session, grade, validated, nonvalidated_grade,
                                                     subtask_identifier, value, date, needs_validation)
                             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)";
                conn.execute(query,
                             &[&self.task,
                               &self.session_user,
                               &self.grade,
                               &self.validated,
                               &self.nonvalidated_grade,
                               &self.subtask_identifier,
                               &self.value,
                               &self.date,
                               &self.needs_validation])
                    .unwrap();
                self.set_id(conn.get_last_id().unwrap());
            }
        }
    }
}

impl MedalObject<Connection> for Grade {
    fn save(&mut self, conn: &Connection) {
        let query = "INSERT OR REPLACE INTO grade (taskgroup, session, grade, validated)
                     VALUES (?1, ?2, ?3, ?4)";

        conn.execute(query, &[&self.taskgroup, &self.user, &self.grade, &self.validated]).unwrap();
    }
}
