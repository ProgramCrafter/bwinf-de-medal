#![cfg(feature = "rusqlite")]

extern crate rusqlite;

use rusqlite::Connection;
use time;
use time::Duration;

use db_conn::{MedalConnection, MedalObject};
use db_objects::*;
use helpers;

trait Queryable {
    fn query_map_one<T, F>(&self, sql: &str, params: &[&rusqlite::types::ToSql], f: F) -> rusqlite::Result<Option<T>>
        where F: FnOnce(&rusqlite::Row) -> T;
}

impl Queryable for Connection {
    fn query_map_one<T, F>(&self, sql: &str, params: &[&rusqlite::types::ToSql], f: F) -> rusqlite::Result<Option<T>>
        where F: FnOnce(&rusqlite::Row) -> T {
        let mut stmt = self.prepare(sql)?;
        let mut rows = stmt.query(params)?;
        match rows.next() {
            None => Ok(None),
            Some(Err(e)) => Err(e),
            Some(Ok(row)) => Ok(Some(f(&row))),
        }
    }
}

impl MedalConnection for Connection {
    fn dbtype(&self) -> &'static str { "sqlite" }

    fn migration_already_applied(&self, name: &str) -> bool {
        let create_string = "CREATE TABLE IF NOT EXISTS migrations (name TEXT PRIMARY KEY);";
        self.execute(create_string, &[]).unwrap();

        let mut stmt = self.prepare("SELECT name FROM migrations WHERE name = ?1").unwrap();
        stmt.exists(&[&name]).unwrap()
    }

    fn apply_migration(&mut self, name: &str, contents: &str) {
        print!("Applying migration `{}` … ", name);

        let tx = self.transaction().unwrap();

        tx.execute_batch(&contents).unwrap();
        tx.execute("INSERT INTO migrations (name) VALUES (?1)", &[&name]).unwrap();

        tx.commit().unwrap();

        println!("OK.");
    }

    // fn get_session<T: ToSql>(&self, key: T, keyname: &str) -> Option<SessionUser> {
    fn get_session(&self, key: &str) -> Option<SessionUser> {
        let query = "SELECT id, csrf_token, last_login, last_activity, permanent_login, username, password, logincode, email, email_unconfirmed, email_confirmationcode, firstname, lastname, street, zip, city, nation, grade, is_teacher, managed_by, oauth_provider, oauth_foreign_id, salt FROM session_user WHERE session_token = ?1";
        let session = self.query_map_one(query, &[&key], |row| SessionUser { id: row.get(0),
                                                                             session_token: Some(key.to_string()),
                                                                             csrf_token: row.get(1),
                                                                             last_login: row.get(2),
                                                                             last_activity: row.get(3),
                                                                             permanent_login: row.get(4),

                                                                             username: row.get(5),
                                                                             password: row.get(6),
                                                                             salt: row.get(22),
                                                                             logincode: row.get(7),
                                                                             email: row.get(8),
                                                                             email_unconfirmed: row.get(9),
                                                                             email_confirmationcode: row.get(10),

                                                                             firstname: row.get(11),
                                                                             lastname: row.get(12),
                                                                             street: row.get(13),
                                                                             zip: row.get(14),
                                                                             city: row.get(15),
                                                                             nation: row.get(16),
                                                                             grade: row.get(17),

                                                                             is_teacher: row.get(18),
                                                                             managed_by: row.get(19),

                                                                             oauth_provider: row.get(20),
                                                                             oauth_foreign_id: row.get(21) })
                          .ok()??;

        let duration = if session.permanent_login { Duration::days(90) } else { Duration::minutes(90) };
        let now = time::get_time();
        if let Some(last_activity) = session.last_activity {
            if now - last_activity < duration {
                self.execute("UPDATE session_user SET last_activity = ?1 WHERE id = ?2", &[&now, &session.id]).unwrap();
                Some(session)
            } else {
                // Session timed out
                // Should remove session token from session_user
                None
            }
        } else {
            // last_activity undefined
            // TODO: What should happen here?
            None
        }
    }
    fn save_session(&self, session: SessionUser) {
        self.execute("UPDATE session_user SET
                      username = ?1,
                      password = ?2,
                      salt = ?3,
                      logincode = ?4,
                      firstname = ?5,
                      lastname = ?6,
                      street = ?7,
                      zip = ?8,
                      city = ?9,
                      grade = ?10,
                      is_teacher = ?11
                      WHERE id = ?12",
                     &[&session.username,
                       &session.password,
                       &session.salt,
                       &session.logincode,
                       &session.firstname,
                       &session.lastname,
                       &session.street,
                       &session.zip,
                       &session.city,
                       &session.grade,
                       &session.is_teacher,
                       &session.id])
            .unwrap();
    }
    fn new_session(&self, session_token: &str) -> SessionUser {
        let csrf_token = helpers::make_csrf_token();

        let now = time::get_time();
        self.execute("INSERT INTO session_user (session_token, csrf_token, last_activity, permanent_login, grade, is_teacher)
                      VALUES (?1, ?2, ?3, 0, 0, 0)",
                     &[&session_token, &csrf_token, &now])
            .unwrap();
        let id = self.query_row("SELECT last_insert_rowid()", &[], |row| row.get(0)).unwrap();

        SessionUser::minimal(id, session_token.to_owned(), csrf_token)
    }
    fn get_session_or_new(&self, key: &str) -> SessionUser {
        self.get_session(&key).unwrap_or_else(|| self.new_session(&key))
    }

    fn get_user_by_id(&self, user_id: i32) -> Option<SessionUser> {
        let res = self.query_row("SELECT session_token, csrf_token, last_login, last_activity, permanent_login, username, password, logincode, email, email_unconfirmed, email_confirmationcode, firstname, lastname, street, zip, city, nation, grade, is_teacher, managed_by, oauth_provider, oauth_foreign_id, salt FROM session_user WHERE id = ?1", &[&user_id], |row| {
            SessionUser {
                id: user_id,
                session_token: row.get(0),
                csrf_token: row.get(1),
                last_login: row.get(2),
                last_activity: row.get(3),
                permanent_login: row.get(4),

                username: row.get(5),
                password: row.get(6),
                salt: row.get(22),
                logincode: row.get(7),
                email: row.get(8),
                email_unconfirmed: row.get(9),
                email_confirmationcode: row.get(10),

                firstname: row.get(11),
                lastname: row.get(12),
                street: row.get(13),
                zip: row.get(14),
                city: row.get(15),
                nation: row.get(16),
                grade: row.get(17),

                is_teacher: row.get(18),
                managed_by: row.get(19),

                oauth_provider: row.get(20),
                oauth_foreign_id: row.get(21),
            }
        });
        res.ok()
    }

    fn get_user_and_group_by_id(&self, user_id: i32) -> Option<(SessionUser, Option<Group>)> {
        let session = self.get_user_by_id(user_id)?;

        let group_id = match session.managed_by {
            Some(id) => id,
            None => return Some((session, None)),
        };

        let res = self.query_row("SELECT name, groupcode, tag, admin FROM usergroup WHERE id = ?1",
                                 &[&group_id],
                                 |row| Group { id: Some(group_id),
                                               name: row.get(0),
                                               groupcode: row.get(1),
                                               tag: row.get(2),
                                               admin: row.get(3),
                                               members: Vec::new() });
        match res {
            Ok(group) => Some((session, Some(group))),
            _ => Some((session, None)),
        }
    }

    //TODO: use session
    fn login(&self, _session: Option<&str>, username: &str, password: &str) -> Result<String, ()> {
        match self.query_row("SELECT id, password, salt FROM session_user WHERE username = ?1",
                             &[&username],
                             |row| -> (i32, Option<String>, Option<String>) { (row.get(0), row.get(1), row.get(2)) })
        {
            Ok((id, password_hash, salt)) => {
                //password_hash ist das, was in der Datenbank steht
                if helpers::verify_password(&password,
                                            &salt.expect("salt from database empty"),
                                            &password_hash.expect("password from database empty"))
                {
                    // TODO: fail more pleasantly
                    // Login okay, update session now!

                    let session_token = helpers::make_session_token();
                    let csrf_token = helpers::make_session_token();
                    let now = time::get_time();

                    self.execute("UPDATE session_user SET session_token = ?1, csrf_token = ?2, last_login = ?3, last_activity = ?3 WHERE id = ?4", &[&session_token, &csrf_token, &now, &id]).unwrap();

                    Ok(session_token)
                } else {
                    Err(())
                }
            }
            _ => Err(()),
        }
    }

    //TODO: use session
    fn login_with_code(&self, _session: Option<&str>, logincode: &str) -> Result<String, ()> {
        match self.query_row("SELECT id FROM session_user WHERE logincode = ?1", &[&logincode], |row| -> i32 {
                      row.get(0)
                  }) {
            Ok(id) => {
                // Login okay, update session now!

                let session_token = helpers::make_session_token();
                let csrf_token = helpers::make_csrf_token();
                let now = time::get_time();

                self.execute("UPDATE session_user SET session_token = ?1, csrf_token = ?2, last_login = ?3, last_activity = ?3 WHERE id = ?4", &[&session_token, &csrf_token, &now, &id]).unwrap();

                Ok(session_token)
            }
            _ => Err(()),
        }
    }

    //TODO: use session
    fn login_foreign(&self, _session: Option<&str>, foreign_id: &str, is_teacher: bool, firstname: &str,
                     lastname: &str)
                     -> Result<String, ()>
    {
        let session_token = helpers::make_session_token();
        let csrf_token = helpers::make_csrf_token();
        let now = time::get_time();

        match self.query_row("SELECT id FROM session_user WHERE oauth_foreign_id = ?1", &[&foreign_id], |row| -> i32 {
                      row.get(0)
                  }) {
            Ok(id) => {
                self.execute("UPDATE session_user SET session_token = ?1, csrf_token = ?2, last_login = ?3, last_activity = ?3 WHERE id = ?4", &[&session_token, &csrf_token, &now, &id]).unwrap();

                Ok(session_token)
            }
            // Add!
            _ => {
                self.execute("INSERT INTO session_user (session_token, csrf_token, last_login, last_activity, permanent_login, grade, is_teacher, oauth_foreign_id, firstname, lastname) VALUES (?1, ?2, ?3, ?3, ?4, ?5, ?6, ?7, ?8, ?9)", &[&session_token, &csrf_token, &now, &false, &0, &is_teacher, &foreign_id, &firstname, &lastname]).unwrap();

                Ok(session_token)
            }
        }
    }

    //TODO: use session
    fn create_user_with_groupcode(&self, _session: Option<&str>, groupcode: &str) -> Result<String, ()> {
        match self.query_row("SELECT id FROM usergroup WHERE groupcode = ?1", &[&groupcode], |row| -> i32 {
                      row.get(0)
                  }) {
            Ok(group_id) => {
                // Login okay, create session_user!

                let session_token = helpers::make_session_token();
                let csrf_token = helpers::make_csrf_token();
                let login_code = helpers::make_login_code(); // TODO: check for collisions
                let now = time::get_time();

                self.execute("INSERT INTO session_user (session_token, csrf_token, last_login, last_activity, permanent_login, logincode, grade, is_teacher, managed_by) VALUES (?1, ?2, ?3, ?3, ?4, ?5, ?6, ?7, ?8)", &[&session_token, &csrf_token, &now, &false, &login_code, &0, &false, &group_id]).unwrap();

                Ok(session_token)
            }
            _ => Err(()),
        }
    }
    
    fn create_group_with_users(&self, mut group: Group) {
        // Generate group ID:
        group.save(self);

        for user in group.members {
            let csrf_token = helpers::make_csrf_token();
            let login_code = helpers::make_login_code(); // TODO: check for collisions
            let now = time::get_time();
            
            self.execute("INSERT INTO session_user (firstname, lastname, csrf_token, permanent_login, logincode, grade, is_teacher, managed_by) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)", &[&user.firstname, &user.lastname, &csrf_token, &false, &login_code, &user.grade, &false, &group.id]).unwrap();
        }
    }

    fn logout(&self, session: &str) {
        self.execute("UPDATE session_user SET session_token = NULL WHERE session_token = ?1", &[&session]).unwrap();
    }

    fn load_submission(&self, session: &SessionUser, task: i32, subtask: Option<&str>) -> Option<Submission> {
        match subtask {
            None => self.query_row("SELECT id, grade, validated, nonvalidated_grade, value, date, needs_validation FROM submission WHERE task = ?1 AND session_user = ?2 ORDER BY id DESC LIMIT 1", &[&task, &session.id], |row| {
                Submission {
                    id: Some(row.get(0)),
                    task: task,
                    session_user: session.id,
                    grade: row.get(1),
                    validated: row.get(2),
                    nonvalidated_grade: row.get(3 ),
                    subtask_identifier: None,
                    value: row.get(4),
                    date: row.get(5),
                    needs_validation: row.get(6),
                }
            }).ok(),
            Some(subtask_id) => self.query_row("SELECT id, grade, validated, nonvalidated_grade, value, date, needs_validation FROM submission WHERE task = ?1 AND session_user = ?2 AND subtask_identifier = ?3 ORDER BY id DESC LIMIT 1", &[&task, &session.id, &subtask_id], |row| {
                Submission {
                    id: Some(row.get(0)),
                    task: task,
                    session_user: session.id,
                    grade: row.get(1),
                    validated: row.get(2),
                    nonvalidated_grade: row.get(3),
                    subtask_identifier: Some(subtask_id.to_string()),
                    value: row.get(4),
                    date: row.get(5),
                    needs_validation: row.get(6),
                }
            }).ok()
        }
    }
    fn submit_submission(&self, mut submission: Submission) {
        submission.save(self);

        let mut grade = self.get_grade_by_submission(submission.id.unwrap());
        if grade.grade.is_none() || submission.grade > grade.grade.unwrap() {
            grade.grade = Some(submission.grade);
            grade.validated = false;
            grade.save(self);
        }
    }
    fn get_grade_by_submission(&self, submission_id: i32) -> Grade {
        self.query_row("SELECT grade.taskgroup, grade.user, grade.grade, grade.validated FROM grade JOIN task ON grade.taskgroup = task.taskgroup JOIN submission ON task.id = submission.task AND grade.user = submission.session_user WHERE submission.id = ?1", &[&submission_id], |row| {
            Grade {
                taskgroup: row.get(0),
                user: row.get(1),
                grade: row.get(2),
                validated: row.get(3),
            }
        }).unwrap_or_else(|_| {
            self.query_row("SELECT task.taskgroup, submission.session_user FROM submission JOIN task ON task.id = submission.task WHERE submission.id = ?1", &[&submission_id], |row| {
                Grade {
                    taskgroup: row.get(0),
                    user: row.get(1),
                    grade: None,
                    validated: false,
                }
            }).unwrap() // should this unwrap?
        })
    }

    fn get_contest_groups_grades(&self, session_id: i32, contest_id: i32)
                                 -> (Vec<String>, Vec<(Group, Vec<(UserInfo, Vec<Grade>)>)>) {
        let mut stmt = self.prepare("SELECT id, name FROM taskgroup WHERE contest = ?1 ORDER BY id ASC").unwrap();
        let tasknames_iter = stmt.query_map(&[&contest_id], |row| {
                                     let x: (i32, String) = (row.get(0), row.get(1));
                                     x
                                 })
                                 .unwrap();

        let tasknames: Vec<(i32, String)> = tasknames_iter.map(|x| x.unwrap()).collect();
        let mut taskindex: ::std::collections::BTreeMap<i32, usize> = ::std::collections::BTreeMap::new();

        let n_tasks = tasknames.len();
        for (index, (i, _)) in tasknames.iter().enumerate() {
            taskindex.insert(*i, index);
        }

        let mut stmt = self.prepare("SELECT grade.taskgroup, grade.user, grade.grade, grade.validated, usergroup.id, usergroup.name, usergroup.groupcode, usergroup.tag, student.id, student.username, student.logincode, student.firstname, student.lastname
                                     FROM grade
                                     JOIN taskgroup ON grade.taskgroup = taskgroup.id
                                     JOIN session_user AS student ON grade.user = student.id
                                     JOIN usergroup ON student.managed_by = usergroup.id
                                     WHERE usergroup.admin = ?1 AND taskgroup.contest = ?2
                                     ORDER BY usergroup.id, student.id, taskgroup.id ASC").unwrap();
        let mut gradeinfo_iter =
            stmt.query_map(&[&session_id, &contest_id], |row| {
                    (Grade { taskgroup: row.get(0), user: row.get(1), grade: row.get(2), validated: row.get(3) },
                     Group { id: Some(row.get(4)),
                             name: row.get(5),
                             groupcode: row.get(6),
                             tag: row.get(7),
                             admin: session_id,
                             members: Vec::new() },
                     UserInfo { id: row.get(8),
                                username: row.get(9),
                                logincode: row.get(10),
                                firstname: row.get(11),
                                lastname: row.get(12) })
                })
                .unwrap();

        if let Some(t /*Ok((grade, mut group, mut userinfo))*/) = gradeinfo_iter.next() {
            let (grade, group, userinfo) = t.unwrap();

            let mut grades: Vec<Grade> = vec![Default::default(); n_tasks];
            let mut users: Vec<(UserInfo, Vec<Grade>)> = Vec::new();
            let mut groups: Vec<(Group, Vec<(UserInfo, Vec<Grade>)>)> = Vec::new();

            let index = grade.taskgroup;
            grades[taskindex[&index]] = grade;

            // TODO: does
            // https://stackoverflow.com/questions/29859892/mutating-an-item-inside-of-nested-loops
            // help to spare all these clones?

            for ggu in gradeinfo_iter {
                if let Ok((g, gr, ui)) = ggu {
                    if gr.id != group.id {
                        users.push((userinfo.clone(), grades));
                        grades = vec![Default::default(); n_tasks];

                        groups.push((group.clone(), users));
                        users = Vec::new();
                    } else if ui.id != userinfo.id {
                        users.push((userinfo.clone(), grades));
                        grades = vec![Default::default(); n_tasks];
                    }
                    let index = g.taskgroup;
                    grades[taskindex[&index]] = g;
                }
            }
            users.push((userinfo, grades));
            groups.push((group, users));

            (tasknames.iter().map(|(_, name)| name.clone()).collect(), groups)
        } else {
            (Vec::new(), Vec::new()) // should those be default filled?
        }
    }
    fn get_contest_user_grades(&self, session_token: &str, contest_id: i32) -> Vec<Grade> {
        let mut stmt = self.prepare("SELECT id, name FROM taskgroup WHERE contest = ?1 ORDER BY id ASC").unwrap();
        let tasknames_iter = stmt.query_map(&[&contest_id], |row| {
                                     let x: (i32, String) = (row.get(0), row.get(1));
                                     x
                                 })
                                 .unwrap();

        let tasknames: Vec<(i32, String)> = tasknames_iter.map(|x| x.unwrap()).collect();
        let mut taskindex: ::std::collections::BTreeMap<i32, usize> = ::std::collections::BTreeMap::new();

        let n_tasks = tasknames.len();
        for (index, (i, _)) in tasknames.iter().enumerate() {
            taskindex.insert(*i, index);
        }

        let mut stmt = self.prepare("SELECT grade.taskgroup, grade.user, grade.grade, grade.validated
                                     FROM grade
                                     JOIN taskgroup ON grade.taskgroup = taskgroup.id
                                     JOIN session_user ON session_user.id = grade.user
                                     WHERE session_user.session_token = ?1 AND taskgroup.contest = ?2
                                     ORDER BY taskgroup.id ASC")
                           .unwrap();
        let gradeinfo_iter = stmt.query_map(&[&session_token, &contest_id], |row| Grade { taskgroup: row.get(0),
                                                                                          user: row.get(1),
                                                                                          grade: row.get(2),
                                                                                          validated: row.get(3) })
                                 .unwrap();

        let mut grades: Vec<Grade> = vec![Default::default(); n_tasks];

        for g in gradeinfo_iter {
            let g = g.unwrap();
            let index = g.taskgroup;
            grades[taskindex[&index]] = g;
        }

        grades
    }

    fn get_taskgroup_user_grade(&self, session_token: &str, taskgroup_id: i32) -> Grade {
        let mut stmt = self.prepare("SELECT grade.taskgroup, grade.user, grade.grade, grade.validated
                                     FROM grade
                                     JOIN session_user ON session_user.id = grade.user
                                     WHERE session_user.session_token = ?1 AND grade.taskgroup = ?2")
                           .unwrap();
        let gradeinfo_iter = stmt.query_map(&[&session_token, &taskgroup_id], |row| Grade { taskgroup: row.get(0),
                                                                                            user: row.get(1),
                                                                                            grade: row.get(2),
                                                                                            validated: row.get(3) })
                                 .unwrap();

        let grade = gradeinfo_iter.map(|t| t.unwrap_or_default()).next().unwrap_or_default();
        grade
    }

    fn get_contest_list(&self) -> Vec<Contest> {
        let mut stmt =
            self.prepare("SELECT id, location, filename, name, duration, public, start_date, end_date FROM contest")
                .unwrap();
        let res = stmt.query_map(&[], |row| Contest { id: Some(row.get(0)),
                                                      location: row.get(1),
                                                      filename: row.get(2),
                                                      name: row.get(3),
                                                      duration: row.get(4),
                                                      public: row.get(5),
                                                      start: row.get(6),
                                                      end: row.get(7),
                                                      taskgroups: Vec::new() })
                      .unwrap()
                      .filter_map(|row| row.ok())
                      .collect();
        res
    }

    fn get_contest_by_id(&self, contest_id: i32) -> Contest {
        self.query_row("SELECT location, filename, name, duration, public, start_date, end_date FROM contest WHERE id = ?1", &[&contest_id], |row| {
            Contest {
                id: Some(contest_id),
                location: row.get(0),
                filename: row.get(1),
                name: row.get(2),
                duration: row.get(3),
                public: row.get(4),
                start: row.get(5),
                end: row.get(6),
                taskgroups: Vec::new(),
            }
        }).unwrap()
    }

    fn get_contest_by_id_complete(&self, contest_id: i32) -> Contest {
        let mut stmt = self.prepare("SELECT contest.location, contest.filename, contest.name, contest.duration, contest.public, contest.start_date, contest.end_date, taskgroup.id, taskgroup.name, task.id, task.location, task.stars FROM contest JOIN taskgroup ON contest.id = taskgroup.contest JOIN task ON taskgroup.id = task.taskgroup WHERE contest.id = ?1").unwrap();

        let mut taskgroupcontest_iter =
            stmt.query_map(&[&contest_id], |row| {
                    (Contest { id: Some(contest_id),
                               location: row.get(0),
                               filename: row.get(1),
                               name: row.get(2),
                               duration: row.get(3),
                               public: row.get(4),
                               start: row.get(5),
                               end: row.get(6),
                               taskgroups: Vec::new() },
                     Taskgroup { id: Some(row.get(7)), contest: contest_id, name: row.get(8), tasks: Vec::new() },
                     Task { id: Some(row.get(9)), taskgroup: row.get(7), location: row.get(10), stars: row.get(11) })
                })
                .unwrap();

        let (mut contest, mut taskgroup, task) = taskgroupcontest_iter.next().unwrap().unwrap();
        taskgroup.tasks.push(task);
        for tgc in taskgroupcontest_iter {
            if let Ok((_, tg, t)) = tgc {
                if tg.id != taskgroup.id {
                    contest.taskgroups.push(taskgroup);
                    taskgroup = tg;
                }
                taskgroup.tasks.push(t);
            }
        }
        contest.taskgroups.push(taskgroup);
        contest
    }

    fn get_contest_by_id_partial(&self, contest_id: i32) -> Contest {
        let mut stmt = self.prepare("SELECT contest.location, contest.filename, contest.name, contest.duration, contest.public, contest.start_date, contest.end_date, taskgroup.id, taskgroup.name FROM contest JOIN taskgroup ON contest.id = taskgroup.contest WHERE contest.id = ?1").unwrap();

        let mut taskgroupcontest_iter =
            stmt.query_map(&[&contest_id], |row| {
                    (Contest { id: Some(contest_id),
                               location: row.get(0),
                               filename: row.get(1),
                               name: row.get(2),
                               duration: row.get(3),
                               public: row.get(4),
                               start: row.get(5),
                               end: row.get(6),
                               taskgroups: Vec::new() },
                     Taskgroup { id: Some(row.get(7)), contest: contest_id, name: row.get(8), tasks: Vec::new() })
                })
                .unwrap();

        let (mut contest, taskgroup) = taskgroupcontest_iter.next().unwrap().unwrap();
        contest.taskgroups.push(taskgroup);
        for tgc in taskgroupcontest_iter {
            if let Ok((_, tg)) = tgc {
                contest.taskgroups.push(tg);
            }
        }
        contest
    }

    fn get_participation(&self, session: &str, contest_id: i32) -> Option<Participation> {
        self.query_row("SELECT user, start_date FROM participation JOIN session_user ON session_user.id = user WHERE session_user.session_token = ?1 AND contest = ?2", &[&session, &contest_id], |row| {
            Participation {
                contest: contest_id,
                user: row.get(0),
                start: row.get(1)
            }
        }).ok()
    }
    fn new_participation(&self, session: &str, contest_id: i32) -> Result<Participation, ()> {
        match self.query_row("SELECT user, start_date FROM participation JOIN session_user ON session_user.id = user WHERE session_user.session_token = ?1 AND contest = ?2", &[&session, &contest_id], |_| {}) {
            Ok(()) => Err(()),
            Err(_) => {
                let now = time::get_time();
                self.execute(
                    "INSERT INTO participation (contest, user, start_date)
                     SELECT ?1, id, ?2 FROM session_user WHERE session_token = ?3",
                     &[&contest_id, &now, &session]).unwrap();

                Ok(self.get_participation(session, contest_id).unwrap()) // TODO: This errors if not logged in …
            }
        }
    }
    fn get_task_by_id(&self, task_id: i32) -> Task {
        self.query_row("SELECT location, stars, taskgroup FROM task WHERE id = ?1", &[&task_id], |row| {
                Task { id: Some(task_id), taskgroup: row.get(2), location: row.get(0), stars: row.get(1) }
            })
            .unwrap()
    }
    fn get_task_by_id_complete(&self, task_id: i32) -> (Task, Taskgroup, Contest) {
        self.query_row(
            "SELECT task.location, task.stars, taskgroup.id, taskgroup.name, contest.id, contest.location, contest.filename, contest.name, contest.duration, contest.public, contest.start_date, contest.end_date FROM contest JOIN taskgroup ON taskgroup.contest = contest.id JOIN task ON task.taskgroup = taskgroup.id WHERE task.id = ?1",
            &[&task_id],
            |row| {
                (Task {
                    id: Some(task_id),
                    taskgroup: row.get(2),
                    location: row.get(0),
                    stars: row.get(1),
                }, Taskgroup {
                    id: Some(row.get(2)),
                    contest: row.get(4),
                    name: row.get(3),
                    tasks: Vec::new(),
                }, Contest {
                    id: Some(row.get(4)),
                    location: row.get(5),
                    filename: row.get(6),
                    name: row.get(7),
                    duration: row.get(8),
                    public: row.get(9),
                    start: row.get(10),
                    end: row.get(11),
                    taskgroups: Vec::new(),
                })
            }).unwrap()
    }

    fn get_submission_to_validate(&self, tasklocation: &str, subtask: Option<&str>) -> i32 {
        match subtask {
            Some(st) => self.query_row("SELECT id FROM submission JOIN task ON submission.task = task.id WHERE task.location = ?1  AND subtask_identifier = ?2 AND needs_validation = 1 LIMIT 1", &[&tasklocation, &st], |row| {row.get(0)}).unwrap(),
            None => self.query_row("SELECT id FROM submission JOIN task ON submission.task = task.id WHERE task.location = ?1 AND needs_validation = 1 LIMIT 1", &[&tasklocation], |row| {row.get(0)}).unwrap(),
        }
    }

    fn find_next_submission_to_validate(&self, userid: i32, taskgroupid: i32) {
        let (id, validated) : (i32, bool) = self.query_row("SELECT id, validated FROM submission JOIN task ON submission.task = task.id WHERE task.taskgroup = ?1 AND submission.session_user = ?2 ORDER BY value DESC id DESC LIMIT 1", &[&taskgroupid, &userid], |row| {(row.get(0), row.get(1))}).unwrap();;
        if !validated {
            self.execute("UPDATE submission SET needs_validation = 1 WHERE id = ?1", &[&id]).unwrap();
        }
    }

    fn add_group(&self, group: &mut Group) { group.save(self); }

    fn get_groups(&self, session_id: i32) -> Vec<Group> {
        let mut stmt = self.prepare("SELECT id, name, groupcode, tag FROM usergroup WHERE admin = ?1").unwrap();
        let res = stmt.query_map(&[&session_id], |row| Group { id: Some(row.get(0)),
                                                               name: row.get(1),
                                                               groupcode: row.get(2),
                                                               tag: row.get(3),
                                                               admin: session_id,
                                                               members: Vec::new() })
                      .unwrap()
                      .filter_map(|row| row.ok())
                      .collect();
        res
    }
    fn get_groups_complete(&self, _session_id: i32) -> Vec<Group> {
        unimplemented!();
    }
    fn get_group_complete(&self, group_id: i32) -> Option<Group> {
        let mut group = self.query_row("SELECT name, groupcode, tag, admin FROM usergroup WHERE id  = ?1",
                                       &[&group_id],
                                       |row| Group { id: Some(group_id),
                                                     name: row.get(0),
                                                     groupcode: row.get(1),
                                                     tag: row.get(2),
                                                     admin: row.get(3),
                                                     members: Vec::new() })
                            .unwrap(); // TODO handle error

        let mut stmt = self.prepare("SELECT id, session_token, csrf_token, last_login, last_activity, permanent_login, username, password, logincode, email, email_unconfirmed, email_confirmationcode, firstname, lastname, street, zip, city, nation, grade, is_teacher, oauth_provider, oauth_foreign_id, salt FROM session_user WHERE managed_by = ?1").unwrap();
        let rows = stmt.query_map(&[&group_id], |row| SessionUser { id: row.get(0),
                                                                    session_token: row.get(1),
                                                                    csrf_token: row.get(2),
                                                                    last_login: row.get(3),
                                                                    last_activity: row.get(4),
                                                                    permanent_login: row.get(5),

                                                                    username: row.get(6),
                                                                    password: row.get(7),
                                                                    salt: row.get(22),
                                                                    logincode: row.get(8),
                                                                    email: row.get(9),
                                                                    email_unconfirmed: row.get(10),
                                                                    email_confirmationcode: row.get(11),

                                                                    firstname: row.get(12),
                                                                    lastname: row.get(13),
                                                                    street: row.get(14),
                                                                    zip: row.get(15),
                                                                    city: row.get(16),
                                                                    nation: row.get(17),
                                                                    grade: row.get(18),

                                                                    is_teacher: row.get(19),
                                                                    managed_by: Some(group_id),

                                                                    oauth_provider: row.get(20),
                                                                    oauth_foreign_id: row.get(21) })
                       .unwrap();

        for user in rows {
            group.members.push(user.unwrap());
        }
        Some(group)
    }
}

impl MedalObject<Connection> for Task {
    fn save(&mut self, conn: &Connection) {
        conn.query_row("SELECT id FROM task WHERE taskgroup = ?1 AND location = ?2",
                       &[&self.taskgroup, &self.location],
                       |row| row.get(0))
            .and_then(|id| {
                self.set_id(id);
                Ok(())
            })
            .unwrap_or(()); // Err means no entry yet and is expected result

        let id = match self.get_id() {
            Some(id) => {
                conn.execute(
                             "UPDATE task SET taskgroup = ?1, location = ?2, stars = ?3
                     WHERE id = ?4",
                             &[&self.taskgroup, &self.location, &self.stars, &id],
                )
                    .unwrap();
                id
            }
            None => {
                conn.execute(
                             "INSERT INTO task (taskgroup, location, stars)
                     VALUES (?1, ?2, ?3)",
                             &[&self.taskgroup, &self.location, &self.stars],
                )
                    .unwrap();
                conn.query_row("SELECT last_insert_rowid()", &[], |row| row.get(0)).unwrap()
            }
        };
        self.set_id(id);
    }
}

impl MedalObject<Connection> for Taskgroup {
    fn save(&mut self, conn: &Connection) {
        conn.query_row("SELECT id FROM taskgroup WHERE contest = ?1 AND name = ?2",
                       &[&self.contest, &self.name],
                       |row| row.get(0))
            .and_then(|id| {
                self.set_id(id);
                Ok(())
            })
            .unwrap_or(()); // Err means no entry yet and is expected result

        let id = match self.get_id() {
            Some(id) => {
                conn.execute(
                             "UPDATE taskgroup SET contest = ?1, name = ?2
                     WHERE id = ?3",
                             &[&self.contest, &self.name, &id],
                )
                    .unwrap();
                id
            }
            None => {
                conn.execute(
                             "INSERT INTO taskgroup (contest, name)
                     VALUES (?1, ?2)",
                             &[&self.contest, &self.name],
                )
                    .unwrap();
                conn.query_row("SELECT last_insert_rowid()", &[], |row| row.get(0)).unwrap()
            }
        };
        self.set_id(id);
        for mut task in &mut self.tasks {
            task.taskgroup = id;
            task.save(conn);
        }
    }
}

impl MedalObject<Connection> for Contest {
    fn save(&mut self, conn: &Connection) {
        conn.query_row("SELECT id FROM contest WHERE location = ?1 AND filename = ?2",
                       &[&self.location, &self.filename],
                       |row| row.get(0))
            .and_then(|id| {
                self.set_id(id);
                Ok(())
            })
            .unwrap_or(()); // Err means no entry yet and is expected result

        let id =
            match self.get_id() {
                Some(id) => {
                    conn.execute(
                                 "UPDATE contest SET location = ?1,filename = ?2,
                     name = ?3, duration = ?4, public = ?5, start_date = ?6,
                     end_date = ?7 WHERE id = ?8",
                                 &[
                        &self.location,
                        &self.filename,
                        &self.name,
                        &self.duration,
                        &self.public,
                        &self.start,
                        &self.end,
                        &id,
                    ],
                    )
                        .unwrap();
                    id
                }
                None => {
                    conn.execute(
                    "INSERT INTO contest (location, filename, name, duration, public, start_date, end_date)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    &[&self.location, &self.filename, &self.name,
                      &self.duration, &self.public, &self.start, &self.end]).unwrap();
                    conn.query_row("SELECT last_insert_rowid()", &[], |row| row.get(0)).unwrap()
                }
            };
        self.set_id(id);
        for mut taskgroup in &mut self.taskgroups {
            taskgroup.contest = id;
            taskgroup.save(conn);
        }
    }
}

impl MedalObject<Connection> for Grade {
    fn save(&mut self, conn: &Connection) {
        conn.execute("INSERT OR REPLACE INTO grade (taskgroup, user, grade, validated) VALUES (?1, ?2, ?3, ?4)",
                     &[&self.taskgroup, &self.user, &self.grade, &self.validated])
            .unwrap();
    }
}

impl MedalObject<Connection> for Participation {
    fn save(&mut self, conn: &Connection) {
        conn.execute("INSERT INTO participation (contest, user, start_date) VALUES (?1, ?2, ?3)",
                     &[&self.contest, &self.user, &self.start])
            .unwrap();
    }
}

impl MedalObject<Connection> for Submission {
    fn save(&mut self, conn: &Connection) {
        match self.get_id() {
            Some(_id) => unimplemented!(),
            None => {
                conn.execute("INSERT INTO submission (task, session_user, grade, validated, nonvalidated_grade, subtask_identifier, value, date, needs_validation) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)", &[&self.task, &self.session_user, &self.grade, &self.validated, &self.nonvalidated_grade, &self.subtask_identifier, &self.value, &self.date, &self.needs_validation]).unwrap();
                self.set_id(conn.query_row("SELECT last_insert_rowid()", &[], |row| row.get(0)).unwrap());
            }
        }
    }
}

impl MedalObject<Connection> for Group {
    fn save(&mut self, conn: &Connection) {
        match self.get_id() {
            Some(_id) => unimplemented!(),
            None => {
                conn.execute("INSERT INTO usergroup (name, groupcode, tag, admin) VALUES (?1, ?2, ?3, ?4)",
                             &[&self.name, &self.groupcode, &self.tag, &self.admin])
                    .unwrap();
                self.set_id(conn.query_row("SELECT last_insert_rowid()", &[], |row| row.get(0)).unwrap());
            }
        }
    }
}
