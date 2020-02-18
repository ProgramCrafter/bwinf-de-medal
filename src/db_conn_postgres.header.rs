#![cfg(feature = "postgres")]

extern crate postgres;

use postgres::Connection;
use time;
use time::Duration;

use db_conn::{MedalConnection, MedalObject};
use db_objects::*;
use helpers;

trait Queryable {
    fn query_map_one<T, F>(&self, sql: &str, params: &[&dyn postgres::types::ToSql], f: F)
                           -> postgres::Result<Option<T>>
        where F: FnOnce(postgres::rows::Row<'_>) -> T;
    fn query_map_many<T, F>(&self, sql: &str, params: &[&dyn postgres::types::ToSql], f: F) -> postgres::Result<Vec<T>>
        where F: FnMut(postgres::rows::Row<'_>) -> T;
    fn exists(&self, sql: &str, params: &[&dyn postgres::types::ToSql]) -> bool;
    fn get_last_id(&self) -> Option<i32>;
}

impl Queryable for Connection {
    fn query_map_one<T, F>(&self, sql: &str, params: &[&dyn postgres::types::ToSql], f: F)
                           -> postgres::Result<Option<T>>
        where F: FnOnce(postgres::rows::Row<'_>) -> T {
        let rows = self.query(sql, params)?;

        Ok(rows.iter().next().map(f))
    }

    fn query_map_many<T, F>(&self, sql: &str, params: &[&dyn postgres::types::ToSql], f: F) -> postgres::Result<Vec<T>>
        where F: FnMut(postgres::rows::Row<'_>) -> T {
        Ok(self.query(sql, params)?.iter().map(f).collect())
    }

    fn exists(&self, sql: &str, params: &[&dyn postgres::types::ToSql]) -> bool {
        let stmt = self.prepare(sql).unwrap();
        !stmt.query(params).unwrap().is_empty()
    }

    fn get_last_id(&self) -> Option<i32> {
        self.query("SELECT lastval()", &[]).unwrap().iter().next().map(|row| {
                                                                      let r: i64 = row.get(0);
                                                                      r as i32
                                                                  })
    }
    // Empty line intended
}

impl MedalObject<Connection> for Submission {
    fn save(&mut self, conn: &Connection) {
        match self.get_id() {
            Some(_id) => unimplemented!(),
            None => {
                let query = "INSERT INTO submission (task, session, grade, validated, nonvalidated_grade,
                                                     subtask_identifier, value, date, needs_validation)
                             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)";
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
        let query = "INSERT INTO grade (taskgroup, session, grade, validated)
                     VALUES ($1, $2, $3, $4)
                     ON CONFLICT ON CONSTRAINT grade_pkey DO UPDATE SET grade = excluded.grade, validated = excluded.validated";
        conn.execute(query, &[&self.taskgroup, &self.user, &self.grade, &self.validated]).unwrap();
    }
}
