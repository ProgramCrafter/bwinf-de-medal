#![cfg(feature = "rusqlite_new")]

extern crate rusqlite;

use rusqlite::Connection;
use time;
use time::Duration;

use db_conn::{MedalConnection, MedalObject};
use db_objects::*;
use helpers;

trait Queryable {
    fn query_map_one<T, F>(&self, sql: &str, params: &[&dyn rusqlite::types::ToSql], f: F)
                           -> rusqlite::Result<Option<T>>
        where F: FnOnce(&rusqlite::Row) -> T;
    fn query_map_many<T, F>(&self, sql: &str, params: &[&dyn rusqlite::types::ToSql], f: F) -> rusqlite::Result<Vec<T>>
        where F: FnMut(&rusqlite::Row) -> T;
    fn exists(&self, sql: &str, params: &[&dyn rusqlite::types::ToSql]) -> bool;
    fn get_last_id(&self) -> Option<i32>;
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
