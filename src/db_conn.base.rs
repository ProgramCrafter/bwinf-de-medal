impl MedalObject<Connection> for Participation {
    fn save(&mut self, conn: &Connection) {
        let query = "INSERT INTO participation (contest, session, start_date)
                     VALUES ($1, $2, $3)";
        conn.execute(query, &[&self.contest, &self.user, &self.start]).unwrap();
    }
}

impl MedalObject<Connection> for Group {
    fn save(&mut self, conn: &Connection) {
        match self.get_id() {
            Some(_id) => unimplemented!(),
            None => {
                let query = "INSERT INTO usergroup (name, groupcode, tag, admin)
                             VALUES ($1, $2, $3, $4)";
                conn.execute(query, &[&self.name, &self.groupcode, &self.tag, &self.admin]).unwrap();
                self.set_id(conn.get_last_id().unwrap());
            }
        }
    }
}

impl MedalObject<Connection> for Task {
    fn save(&mut self, conn: &Connection) {
        let query = "SELECT id
                     FROM task
                     WHERE taskgroup = $1
                     AND location = $2";
        conn.query_map_one(query, &[&self.taskgroup, &self.location], |row| row.get(0))
            .unwrap_or(None)
            .and_then(|id| {
                self.set_id(id);
                Some(())
            })
            .unwrap_or(()); // Err means no entry yet and is expected result

        let id = match self.get_id() {
            Some(id) => {
                let query = "UPDATE task
                             SET taskgroup = $1, location = $2, stars = $3
                             WHERE id = $4";
                conn.execute(query, &[&self.taskgroup, &self.location, &self.stars, &id]).unwrap();
                id
            }
            None => {
                let query = "INSERT INTO task (taskgroup, location, stars)
                             VALUES ($1, $2, $3)";
                conn.execute(query, &[&self.taskgroup, &self.location, &self.stars]).unwrap();
                conn.get_last_id().unwrap()
            }
        };
        self.set_id(id);
    }
}

impl MedalObject<Connection> for Taskgroup {
    fn save(&mut self, conn: &Connection) {
        if let Some(first_task) = self.tasks.get(0) {
            let query = "SELECT taskgroup.id
                         FROM taskgroup
                         JOIN task
                         ON task.taskgroup = taskgroup.id
                         WHERE contest = $1
                         AND task.location = $2";
            conn.query_map_one(query, &[&self.contest, &first_task.location], |row| row.get(0))
                .unwrap_or(None)
                .and_then(|id| {
                    self.set_id(id);
                    Some(())
                })
                .unwrap_or(()); // Err means no entry yet and is expected result
        }

        let id = match self.get_id() {
            Some(id) => {
                let query = "UPDATE taskgroup
                             SET contest = $1, name = $2, active = $3, positionalnumber = $4
                             WHERE id = $5";
                conn.execute(query, &[&self.contest, &self.name, &self.active, &self.positionalnumber, &id]).unwrap();
                id
            }
            None => {
                let query = "INSERT INTO taskgroup (contest, name, active, positionalnumber)
                             VALUES ($1, $2, $3, $4)";
                conn.execute(query, &[&self.contest, &self.name, &self.active, &self.positionalnumber]).unwrap();
                conn.get_last_id().unwrap()
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
        let query = "SELECT id
                     FROM contest
                     WHERE location = $1
                     AND filename = $2";
        conn.query_map_one(query, &[&self.location, &self.filename], |row| row.get(0))
            .unwrap_or(None)
            .and_then(|id| {
                self.set_id(id);
                Some(())
            })
            .unwrap_or(()); // Err means no entry yet and is expected result

        let id = match self.get_id() {
            Some(id) => {
                let query = "UPDATE contest
                             SET location = $1,filename = $2, name = $3, duration = $4, public = $5, start_date = $6,
                                 end_date = $7, min_grade = $8, max_grade = $9, positionalnumber = $10
                             WHERE id = $11";
                conn.execute(query,
                             &[&self.location,
                               &self.filename,
                               &self.name,
                               &self.duration,
                               &self.public,
                               &self.start,
                               &self.end,
                               &self.min_grade,
                               &self.max_grade,
                               &self.positionalnumber,
                               &id])
                    .unwrap();
                id
            }
            None => {
                let query = "INSERT INTO contest (location, filename, name, duration, public, start_date, end_date,
                                                  min_grade, max_grade, positionalnumber)
                             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)";
                conn.execute(query,
                             &[&self.location,
                               &self.filename,
                               &self.name,
                               &self.duration,
                               &self.public,
                               &self.start,
                               &self.end,
                               &self.min_grade,
                               &self.max_grade,
                               &self.positionalnumber])
                    .unwrap();
                conn.get_last_id().unwrap()
            }
        };
        self.set_id(id);
        for mut taskgroup in &mut self.taskgroups {
            taskgroup.contest = id;
            taskgroup.save(conn);
        }
    }
}

impl MedalConnection for Connection {
    fn dbtype(&self) -> &'static str { "postgres" }

    fn migration_already_applied(&self, name: &str) -> bool {
        let create_string = "CREATE TABLE IF NOT EXISTS migrations (name TEXT PRIMARY KEY);";
        self.execute(create_string, &[]).unwrap();

        let query = "SELECT name FROM migrations WHERE name = $1";
        self.exists(query, &[&name])
    }

    fn apply_migration(&mut self, name: &str, contents: &str) {
        print!("Applying migration `{}` … ", name);

        let tx = self.transaction().unwrap();

        tx.batch_execute(&contents).unwrap();
        tx.execute("INSERT INTO migrations (name) VALUES ($1)", &[&name]).unwrap();

        tx.commit().unwrap();

        println!("OK.");
    }

    // fn get_session<T: ToSql>(&self, key: T, keyname: &str) -> Option<SessionUser> {
    fn get_session(&self, key: &str) -> Option<SessionUser> {
        let query = "SELECT id, csrf_token, last_login, last_activity, permanent_login, username, password, logincode,
                            email, email_unconfirmed, email_confirmationcode, firstname, lastname, street, zip, city,
                            nation, grade, sex, is_teacher, managed_by, oauth_provider, oauth_foreign_id, salt
                     FROM session
                     WHERE session_token = $1";
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
                                                                             sex: row.get(18),

                                                                             is_teacher: row.get(19),
                                                                             managed_by: row.get(20),

                                                                             oauth_provider: row.get(21),
                                                                             oauth_foreign_id: row.get(22) })
                          .ok()??;

        let duration = if session.permanent_login { Duration::days(90) } else { Duration::minutes(90) };
        let now = time::get_time();

        if let Some(last_activity) = session.last_activity {
            if now - last_activity < duration {
                let query = "UPDATE session
                             SET last_activity = $1
                             WHERE id = $2";
                self.execute(query, &[&now, &session.id]).unwrap();
                return Some(session);
            } else {
                // Session timed out
                // Should remove session token from session
                return None;
            }
        }
        // last_activity undefined
        // TODO: What should happen here?
        None
    }
    fn save_session(&self, session: SessionUser) {
        self.execute("UPDATE session
                      SET username = $1,
                          password = $2,
                          salt = $3,
                          logincode = $4,
                          firstname = $5,
                          lastname = $6,
                          street = $7,
                          zip = $8,
                          city = $9,
                          grade = $10,
                          sex = $11,
                          is_teacher = $12
                      WHERE id = $13",
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
                       &session.sex,
                       &session.is_teacher,
                       &session.id])
            .unwrap();
    }
    fn new_session(&self, session_token: &str) -> SessionUser {
        let csrf_token = helpers::make_csrf_token();

        let now = time::get_time();
        let query = "INSERT INTO session (session_token, csrf_token, last_activity, permanent_login, grade, sex,
                                          is_teacher)
                     VALUES ($1, $2, $3, $4, $5, $6, $7)";
        self.execute(query, &[&session_token, &csrf_token, &now, &false, &0, &None::<i32>, &false]).unwrap();

        let id = self.get_last_id().expect("Expected to get last row id");

        SessionUser::minimal(id, session_token.to_owned(), csrf_token)
    }
    fn get_session_or_new(&self, key: &str) -> SessionUser {
        let query = "UPDATE session
                     SET session_token = $1
                     WHERE session_token = $2";
        self.get_session(&key).ensure_alive().unwrap_or_else(|| {
                                                 // TODO: Factor this out in own function
                                                 // TODO: Should a new session key be generated every time?
                                                 self.execute(query, &[&Option::<String>::None, &key]).unwrap();
                                                 self.new_session(&key)
                                             })
    }

    fn get_user_by_id(&self, user_id: i32) -> Option<SessionUser> {
        let query = "SELECT session_token, csrf_token, last_login, last_activity, permanent_login, username, password,
                            logincode, email, email_unconfirmed, email_confirmationcode, firstname, lastname, street,
                            zip, city, nation, grade, sex, is_teacher, managed_by, oauth_provider, oauth_foreign_id,
                            salt
                     FROM session
                     WHERE id = $1";
        self.query_map_one(query, &[&user_id], |row| SessionUser { id: user_id,
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
                                                                   sex: row.get(18),

                                                                   is_teacher: row.get(19),
                                                                   managed_by: row.get(20),

                                                                   oauth_provider: row.get(21),
                                                                   oauth_foreign_id: row.get(22) })
            .ok()?
    }

    fn get_user_and_group_by_id(&self, user_id: i32) -> Option<(SessionUser, Option<Group>)> {
        let session = self.get_user_by_id(user_id)?;

        let group_id = match session.managed_by {
            Some(id) => id,
            None => return Some((session, None)),
        };

        let query = "SELECT name, groupcode, tag, admin
                     FROM usergroup
                     WHERE id = $1";
        let res = self.query_map_one(query, &[&group_id], |row| Group { id: Some(group_id),
                                                                        name: row.get(0),
                                                                        groupcode: row.get(1),
                                                                        tag: row.get(2),
                                                                        admin: row.get(3),
                                                                        members: Vec::new() })
                      .ok()?;
        match res {
            Some(group) => Some((session, Some(group))),
            _ => Some((session, None)),
        }
    }

    //TODO: use session
    fn login(&self, _session: Option<&str>, username: &str, password: &str) -> Result<String, ()> {
        let query = "SELECT id, password, salt
                     FROM session
                     WHERE username = $1";
        self.query_map_one(query, &[&username], |row| {
                let (id, password_hash, salt): (i32, Option<String>, Option<String>) =
                    (row.get(0), row.get(1), row.get(2));

                //password_hash ist das, was in der Datenbank steht
                if helpers::verify_password(&password,
                                            &salt.expect("salt from database empty"),
                                            &password_hash.expect("password from database empty"))
                {
                    // TODO: fail more pleasantly
                    // Login okay, update session now!

                    let session_token = helpers::make_session_token();
                    let csrf_token = helpers::make_csrf_token();
                    let now = time::get_time();

                    let query = "UPDATE session
                                 SET session_token = $1, csrf_token = $2, last_login = $3, last_activity = $3
                                 WHERE id = $4";
                    self.execute(query, &[&session_token, &csrf_token, &now, &id]).unwrap();

                    Ok(session_token)
                } else {
                    Err(())
                }
            })
            .map_err(|_| ())?
            .ok_or(())?
    }

    //TODO: use session
    fn login_with_code(&self, _session: Option<&str>, logincode: &str) -> Result<String, ()> {
        let query = "SELECT id
                     FROM session
                     WHERE logincode = $1";
        self.query_map_one(query, &[&logincode], |row| {
                // Login okay, update session now!
                let id: i32 = row.get(0);

                let session_token = helpers::make_session_token();
                let csrf_token = helpers::make_csrf_token();
                let now = time::get_time();

                let query = "UPDATE session
                             SET session_token = $1, csrf_token = $2, last_login = $3, last_activity = $3
                             WHERE id = $4";
                self.execute(query, &[&session_token, &csrf_token, &now, &id]).unwrap();

                session_token
            })
            .map_err(|_| ())?
            .ok_or(())
    }

    //TODO: use session
    fn login_foreign(&self, _session: Option<&str>, provider_id: &str, foreign_id: &str, is_teacher: bool,
                     firstname: &str, lastname: &str)
                     -> Result<String, ()>
    {
        let session_token = helpers::make_session_token();
        let csrf_token = helpers::make_csrf_token();
        let now = time::get_time();

        let query = "SELECT id
                     FROM session
                     WHERE oauth_foreign_id = $1
                           AND oauth_provider = $2";
        match self.query_map_one(query, &[&foreign_id, &provider_id], |row| -> i32 { row.get(0) }) {
            Ok(Some(id)) => {
                let query = "UPDATE session
                             SET session_token = $1, csrf_token = $2, last_login = $3, last_activity = $3
                             WHERE id = $4";
                self.execute(query, &[&session_token, &csrf_token, &now, &id]).unwrap();

                Ok(session_token)
            }
            // Add!
            _ => {
                let query = "INSERT INTO session (session_token, csrf_token, last_login, last_activity,
                                                  permanent_login, grade, sex, is_teacher, oauth_foreign_id,
                                                  oauth_provider, firstname, lastname)
                             VALUES ($1, $2, $3, $3, $4, $5, $6, $7, $8, $9, $10, $11)";
                self.execute(query,
                             &[&session_token,
                               &csrf_token,
                               &now,
                               &false,
                               &(if is_teacher { 255 } else { 0 }),
                               &None::<i32>,
                               &is_teacher,
                               &foreign_id,
                               &provider_id,
                               &firstname,
                               &lastname])
                    .unwrap();

                Ok(session_token)
            }
        }
    }

    //TODO: use session
    fn create_user_with_groupcode(&self, _session: Option<&str>, groupcode: &str) -> Result<String, ()> {
        let query = "SELECT id
                     FROM usergroup
                     WHERE groupcode = $1";
        let group_id =
            self.query_map_one(query, &[&groupcode], |row| -> i32 { row.get(0) }).map_err(|_| ())?.ok_or(())?;

        // Login okay, create session!
        let session_token = helpers::make_session_token();
        let csrf_token = helpers::make_csrf_token();
        let login_code = helpers::make_login_code(); // TODO: check for collisions
        let now = time::get_time();

        let query = "INSERT INTO session (session_token, csrf_token, last_login, last_activity, permanent_login,
                                          logincode, grade, sex, is_teacher, managed_by)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)";
        self.execute(query,
                     &[&session_token,
                       &csrf_token,
                       &now,
                       &now,
                       &false,
                       &login_code,
                       &0,
                       &None::<i32>,
                       &false,
                       &group_id])
            .unwrap();

        Ok(session_token)
    }

    fn create_group_with_users(&self, mut group: Group) {
        // Generate group ID:
        group.save(self);

        for user in group.members {
            let csrf_token = helpers::make_csrf_token();
            let login_code = helpers::make_login_code(); // TODO: check for collisions

            let query = "INSERT INTO session (firstname, lastname, csrf_token, permanent_login, logincode, grade, sex,
                                              is_teacher, managed_by)
                         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)";
            self.execute(query,
                         &[&user.firstname,
                           &user.lastname,
                           &csrf_token,
                           &false,
                           &login_code,
                           &user.grade,
                           &None::<i32>,
                           &false,
                           &group.id])
                .unwrap();
        }
    }

    fn logout(&self, session: &str) {
        let query = "UPDATE session
                     SET session_token = NULL
                     WHERE session_token = $1";
        self.execute(query, &[&session]).unwrap();
    }

    fn load_submission(&self, session: &SessionUser, task: i32, subtask: Option<&str>) -> Option<Submission> {
        match subtask {
            None => {
                let query = "SELECT id, grade, validated, nonvalidated_grade, value, date, needs_validation
                             FROM submission
                             WHERE task = $1
                             AND session = $2
                             ORDER BY id DESC
                             LIMIT 1";
                self.query_map_one(query, &[&task, &session.id], |row| Submission { id: Some(row.get(0)),
                                                                                    task: task,
                                                                                    session_user: session.id,
                                                                                    grade: row.get(1),
                                                                                    validated: row.get(2),
                                                                                    nonvalidated_grade: row.get(3),
                                                                                    subtask_identifier: None,
                                                                                    value: row.get(4),
                                                                                    date: row.get(5),
                                                                                    needs_validation: row.get(6) })
                    .ok()?
            }
            Some(subtask_id) => {
                let query = "SELECT id, grade, validated, nonvalidated_grade, value, date, needs_validation
                             FROM submission
                             WHERE task = $1
                             AND session = $2
                             AND subtask_identifier = $3
                             ORDER BY id DESC
                             LIMIT 1";
                self.query_map_one(query, &[&task, &session.id, &subtask_id], |row| {
                        Submission { id: Some(row.get(0)),
                                     task: task,
                                     session_user: session.id,
                                     grade: row.get(1),
                                     validated: row.get(2),
                                     nonvalidated_grade: row.get(3),
                                     subtask_identifier: Some(subtask_id.to_string()),
                                     value: row.get(4),
                                     date: row.get(5),
                                     needs_validation: row.get(6) }
                    })
                    .ok()?
            }
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
        let query = "SELECT grade.taskgroup, grade.session, grade.grade, grade.validated
                     FROM grade
                     JOIN task ON grade.taskgroup = task.taskgroup
                     JOIN submission ON task.id = submission.task
                     AND grade.session = submission.session
                     WHERE submission.id = $1";
        self.query_map_one(query, &[&submission_id], |row| Grade { taskgroup: row.get(0),
                                                                   user: row.get(1),
                                                                   grade: row.get(2),
                                                                   validated: row.get(3) })
            .unwrap_or(None)
            .unwrap_or_else(|| {
                let query = "SELECT task.taskgroup, submission.session
                         FROM submission
                         JOIN task ON task.id = submission.task
                         WHERE submission.id = $1";
                self.query_map_one(query, &[&submission_id], |row| Grade { taskgroup: row.get(0),
                                                                           user: row.get(1),
                                                                           grade: None,
                                                                           validated: false })
                    .unwrap()
                    .unwrap() // should this unwrap?
            })
    }

    fn get_contest_groups_grades(&self, session_id: i32, contest_id: i32)
                                 -> (Vec<String>, Vec<(Group, Vec<(UserInfo, Vec<Grade>)>)>) {
        let query = "SELECT id, name
                     FROM taskgroup
                     WHERE contest = $1
                     AND active = $2
                     ORDER BY positionalnumber";
        let tasknames: Vec<(i32, String)> =
            self.query_map_many(query, &[&contest_id, &true], |row| (row.get(0), row.get(1))).unwrap();

        let mut taskindex: ::std::collections::BTreeMap<i32, usize> = ::std::collections::BTreeMap::new();

        let n_tasks = tasknames.len();
        for (index, (i, _)) in tasknames.iter().enumerate() {
            taskindex.insert(*i, index);
        }

        let query = "SELECT grade.taskgroup, grade.session, grade.grade, grade.validated, usergroup.id, usergroup.name,
                            usergroup.groupcode, usergroup.tag, student.id, student.username, student.logincode,
                            student.firstname, student.lastname
                     FROM grade
                     JOIN taskgroup ON grade.taskgroup = taskgroup.id
                     JOIN session AS student ON grade.session = student.id
                     JOIN usergroup ON student.managed_by = usergroup.id
                     WHERE usergroup.admin = $1
                     AND taskgroup.contest = $2
                     AND taskgroup.active = $3
                     ORDER BY usergroup.id, student.id, taskgroup.positionalnumber";
        let gradeinfo =
            self.query_map_many(query, &[&session_id, &contest_id, &true], |row| {
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
        let mut gradeinfo_iter = gradeinfo.iter();

        if let Some(t /*Ok((grade, mut group, mut userinfo))*/) = gradeinfo_iter.next() {
            let (grade, mut group, mut userinfo) = t.clone();

            let mut grades: Vec<Grade> = vec![Default::default(); n_tasks];
            let mut users: Vec<(UserInfo, Vec<Grade>)> = Vec::new();
            let mut groups: Vec<(Group, Vec<(UserInfo, Vec<Grade>)>)> = Vec::new();

            let index = grade.taskgroup;
            grades[taskindex[&index]] = grade;

            for ggu in gradeinfo_iter {
                let (g, gr, ui) = ggu;
                if gr.id != group.id {
                    users.push((userinfo, grades));
                    userinfo = ui.clone();
                    grades = vec![Default::default(); n_tasks];

                    groups.push((group, users));
                    group = gr.clone();
                    users = Vec::new();
                } else if ui.id != userinfo.id {
                    users.push((userinfo, grades));
                    userinfo = ui.clone();
                    grades = vec![Default::default(); n_tasks];
                }
                let index = g.taskgroup;
                grades[taskindex[&index]] = *g;
            }
            users.push((userinfo, grades));
            groups.push((group, users));

            (tasknames.iter().map(|(_, name)| name.clone()).collect(), groups)
        } else {
            (Vec::new(), Vec::new()) // should those be default filled?
        }
    }
    fn get_contest_user_grades(&self, session_token: &str, contest_id: i32) -> Vec<Grade> {
        let query = "SELECT id, name
                     FROM taskgroup
                     WHERE contest = $1
                     AND active = $2
                     ORDER BY positionalnumber";
        let tasknames: Vec<(i32, String)> =
            self.query_map_many(query, &[&contest_id, &true], |row| (row.get(0), row.get(1))).unwrap();
        let mut taskindex: ::std::collections::BTreeMap<i32, usize> = ::std::collections::BTreeMap::new();

        let n_tasks = tasknames.len();
        for (index, (i, _)) in tasknames.iter().enumerate() {
            taskindex.insert(*i, index);
        }

        let query = "SELECT grade.taskgroup, grade.session, grade.grade, grade.validated
                     FROM grade
                     JOIN taskgroup ON grade.taskgroup = taskgroup.id
                     JOIN session ON session.id = grade.session
                     WHERE session.session_token = $1
                     AND taskgroup.contest = $2
                     AND taskgroup.active = $3
                     ORDER BY taskgroup.positionalnumber";
        let gradeinfo =
            self.query_map_many(query, &[&session_token, &contest_id, &true], |row| Grade { taskgroup: row.get(0),
                                                                                     user: row.get(1),
                                                                                     grade: row.get(2),
                                                                                     validated: row.get(3) })
                .unwrap();
        let gradeinfo_iter = gradeinfo.iter();

        let mut grades: Vec<Grade> = vec![Default::default(); n_tasks];

        for g in gradeinfo_iter {
            let index = g.taskgroup;
            grades[taskindex[&index]] = *g;
        }

        grades
    }

    fn get_taskgroup_user_grade(&self, session_token: &str, taskgroup_id: i32) -> Grade {
        let query = "SELECT grade.taskgroup, grade.session, grade.grade, grade.validated
                     FROM grade
                     JOIN session ON session.id = grade.session
                     WHERE session.session_token = $1
                     AND grade.taskgroup = $2";
        self.query_map_one(query, &[&session_token, &taskgroup_id], |row| Grade { taskgroup: row.get(0),
                                                                                  user: row.get(1),
                                                                                  grade: row.get(2),
                                                                                  validated: row.get(3) })
            .unwrap_or(None)
            .unwrap_or_default()
    }

    fn get_contest_list(&self) -> Vec<Contest> {
        let query = "SELECT id, location, filename, name, duration, public, start_date, end_date, min_grade, max_grade,
                            positionalnumber
                     FROM contest
                     ORDER BY positionalnumber";
        self.query_map_many(query, &[], |row| Contest { id: Some(row.get(0)),
                                                        location: row.get(1),
                                                        filename: row.get(2),
                                                        name: row.get(3),
                                                        duration: row.get(4),
                                                        public: row.get(5),
                                                        start: row.get(6),
                                                        end: row.get(7),
                                                        min_grade: row.get(8),
                                                        max_grade: row.get(9),
                                                        positionalnumber: row.get(10),
                                                        taskgroups: Vec::new() })
            .unwrap()
    }

    fn get_contest_by_id(&self, contest_id: i32) -> Contest {
        let query = "SELECT location, filename, name, duration, public, start_date, end_date, min_grade, max_grade
                     FROM contest
                     WHERE id = $1";
        self.query_map_one(query, &[&contest_id], |row| Contest { id: Some(contest_id),
                                                                  location: row.get(0),
                                                                  filename: row.get(1),
                                                                  name: row.get(2),
                                                                  duration: row.get(3),
                                                                  public: row.get(4),
                                                                  start: row.get(5),
                                                                  end: row.get(6),
                                                                  min_grade: row.get(7),
                                                                  max_grade: row.get(8),
                                                                  positionalnumber: None,
                                                                  taskgroups: Vec::new() })
            .unwrap()
            .unwrap() // TODO: Should return Option?
    }

    fn get_contest_by_id_complete(&self, contest_id: i32) -> Contest {
        let query = "SELECT contest.location, contest.filename, contest.name, contest.duration, contest.public,
                            contest.start_date, contest.end_date, contest.min_grade, contest.max_grade, taskgroup.id,
                            taskgroup.name, task.id, task.location, task.stars
                     FROM contest
                     JOIN taskgroup ON contest.id = taskgroup.contest
                     JOIN task ON taskgroup.id = task.taskgroup
                     WHERE contest.id = $1
                     AND taskgroup.active = $2
                     ORDER BY taskgroup.positionalnumber";
        let taskgroupcontest =
            self.query_map_many(query, &[&contest_id, &true], |row| {
                    (Contest { id: Some(contest_id),
                               location: row.get(0),
                               filename: row.get(1),
                               name: row.get(2),
                               duration: row.get(3),
                               public: row.get(4),
                               start: row.get(5),
                               end: row.get(6),
                               min_grade: row.get(7),
                               max_grade: row.get(8),
                               positionalnumber: None,
                               taskgroups: Vec::new() },
                     Taskgroup { id: Some(row.get(9)),
                                 contest: contest_id,
                                 name: row.get(10),
                                 active: true,
                                 positionalnumber: None,
                                 tasks: Vec::new() },
                     Task { id: Some(row.get(11)), taskgroup: row.get(9), location: row.get(12), stars: row.get(13) })
                })
                .unwrap();
        let mut taskgroupcontest_iter = taskgroupcontest.into_iter();

        let (mut contest, mut taskgroup, task) = taskgroupcontest_iter.next().unwrap();
        taskgroup.tasks.push(task);
        for tgc in taskgroupcontest_iter {
            let (_, tg, t) = tgc;
            if tg.id != taskgroup.id {
                contest.taskgroups.push(taskgroup);
                taskgroup = tg;
            }
            taskgroup.tasks.push(t);
        }
        contest.taskgroups.push(taskgroup);
        contest
    }

    fn get_contest_by_id_partial(&self, contest_id: i32) -> Contest {
        let query = "SELECT contest.location, contest.filename, contest.name, contest.duration, contest.public,
                            contest.start_date, contest.end_date, contest.min_grade, contest.max_grade, taskgroup.id,
                            taskgroup.name
                     FROM contest
                     JOIN taskgroup ON contest.id = taskgroup.contest
                     WHERE contest.id = $1
                     AND taskgroup.active = $2";
        let taskgroupcontest = self.query_map_many(query, &[&contest_id, &true], |row| {
                                       (Contest { id: Some(contest_id),
                                                  location: row.get(0),
                                                  filename: row.get(1),
                                                  name: row.get(2),
                                                  duration: row.get(3),
                                                  public: row.get(4),
                                                  start: row.get(5),
                                                  end: row.get(6),
                                                  min_grade: row.get(7),
                                                  max_grade: row.get(8),
                                                  positionalnumber: None,
                                                  taskgroups: Vec::new() },
                                        Taskgroup { id: Some(row.get(9)),
                                                    contest: contest_id,
                                                    name: row.get(10),
                                                    active: true,
                                                    positionalnumber: None,
                                                    tasks: Vec::new() })
                                   })
                                   .unwrap();
        let mut taskgroupcontest_iter = taskgroupcontest.into_iter();

        let (mut contest, taskgroup) = taskgroupcontest_iter.next().unwrap();
        contest.taskgroups.push(taskgroup);
        for tgc in taskgroupcontest_iter {
            let (_, tg) = tgc;
            contest.taskgroups.push(tg);
        }
        contest
    }

    fn get_participation(&self, session: &str, contest_id: i32) -> Option<Participation> {
        let query = "SELECT session, start_date
                     FROM participation
                     JOIN session ON session.id = session
                     WHERE session.session_token = $1
                     AND contest = $2";
        self.query_map_one(query, &[&session, &contest_id], |row| Participation { contest: contest_id,
                                                                                  user: row.get(0),
                                                                                  start: row.get(1) })
            .ok()?
    }
    fn new_participation(&self, session: &str, contest_id: i32) -> Result<Participation, ()> {
        let query = "SELECT session, start_date
                     FROM participation
                     JOIN session ON session.id = session
                     WHERE session.session_token = $1
                     AND contest = $2";
        match self.query_map_one(query, &[&session, &contest_id], |_| {}).map_err(|_| ())? {
            Some(()) => Err(()),
            None => {
                let now = time::get_time();
                self.execute(
                             "INSERT INTO participation (contest, session, start_date)
                     SELECT $1, id, $2 FROM session WHERE session_token = $3",
                             &[&contest_id, &now, &session],
                )
                    .unwrap();

                Ok(self.get_participation(session, contest_id).unwrap()) // TODO: This errors if not logged in …
            }
        }
    }
    fn get_task_by_id(&self, task_id: i32) -> Task {
        let query = "SELECT location, stars, taskgroup
                     FROM task
                     WHERE id = $1";
        self.query_map_one(query, &[&task_id], |row| Task { id: Some(task_id),
                                                            taskgroup: row.get(2),
                                                            location: row.get(0),
                                                            stars: row.get(1) })
            .unwrap()
            .unwrap()
    }
    fn get_task_by_id_complete(&self, task_id: i32) -> (Task, Taskgroup, Contest) {
        let query = "SELECT task.location, task.stars, taskgroup.id, taskgroup.name, taskgroup.active, contest.id,
                            contest.location, contest.filename, contest.name, contest.duration, contest.public,
                            contest.start_date, contest.end_date, contest.min_grade, contest.max_grade
                     FROM contest
                     JOIN taskgroup ON taskgroup.contest = contest.id
                     JOIN task ON task.taskgroup = taskgroup.id
                     WHERE task.id = $1";
        self.query_map_one(query, &[&task_id], |row| {
                (Task { id: Some(task_id), taskgroup: row.get(2), location: row.get(0), stars: row.get(1) },
                 Taskgroup { id: Some(row.get(2)),
                             contest: row.get(5),
                             name: row.get(3),
                             active: row.get(4),
                             positionalnumber: None,
                             tasks: Vec::new() },
                 Contest { id: Some(row.get(5)),
                           location: row.get(6),
                           filename: row.get(7),
                           name: row.get(8),
                           duration: row.get(9),
                           public: row.get(10),
                           start: row.get(11),
                           end: row.get(12),
                           min_grade: row.get(13),
                           max_grade: row.get(14),
                           positionalnumber: None,
                           taskgroups: Vec::new() })
            })
            .unwrap()
            .unwrap()
    }

    fn get_submission_to_validate(&self, tasklocation: &str, subtask: Option<&str>) -> i32 {
        match subtask {
            Some(st) => {
                let query = "SELECT id
                             FROM submission
                             JOIN task ON submission.task = task.id
                             WHERE task.location = $1
                             AND subtask_identifier = $2
                             AND needs_validation = 1
                             LIMIT 1";
                self.query_map_one(query, &[&tasklocation, &st], |row| row.get(0)).unwrap().unwrap()
            }
            None => {
                let query = "SELECT id
                             FROM submission
                             JOIN task ON submission.task = task.id
                             WHERE task.location = $1
                             AND needs_validation = 1
                             LIMIT 1";
                self.query_map_one(query, &[&tasklocation], |row| row.get(0)).unwrap().unwrap()
            }
        }
    }

    fn find_next_submission_to_validate(&self, userid: i32, taskgroupid: i32) {
        let query = "SELECT id, validated
                     FROM submission
                     JOIN task ON submission.task = task.id
                     WHERE task.taskgroup = $1
                     AND submission.session = $2
                     ORDER BY value DESC id DESC
                     LIMIT 1";
        let (id, validated): (i32, bool) =
            self.query_map_one(query, &[&taskgroupid, &userid], |row| (row.get(0), row.get(1))).unwrap().unwrap();
        if !validated {
            let query = "UPDATE submission
                         SET needs_validation = 1
                         WHERE id = $1";
            self.execute(query, &[&id]).unwrap();
        }
    }

    fn add_group(&self, group: &mut Group) { group.save(self); }

    fn get_groups(&self, session_id: i32) -> Vec<Group> {
        let query = "SELECT id, name, groupcode, tag
                     FROM usergroup
                     WHERE admin = $1";
        self.query_map_many(query, &[&session_id], |row| Group { id: Some(row.get(0)),
                                                                 name: row.get(1),
                                                                 groupcode: row.get(2),
                                                                 tag: row.get(3),
                                                                 admin: session_id,
                                                                 members: Vec::new() })
            .unwrap()
    }
    fn get_groups_complete(&self, _session_id: i32) -> Vec<Group> {
        unimplemented!();
    }
    fn get_group_complete(&self, group_id: i32) -> Option<Group> {
        let query = "SELECT name, groupcode, tag, admin
                     FROM usergroup
                     WHERE id  = $1";
        let mut group = self.query_map_one(query, &[&group_id], |row| Group { id: Some(group_id),
                                                                              name: row.get(0),
                                                                              groupcode: row.get(1),
                                                                              tag: row.get(2),
                                                                              admin: row.get(3),
                                                                              members: Vec::new() })
                            .unwrap()
                            .unwrap(); // TODO handle error

        let query = "SELECT id, session_token, csrf_token, last_login, last_activity, permanent_login, username,
                            password, logincode, email, email_unconfirmed, email_confirmationcode, firstname, lastname,
                            street, zip, city, nation, grade, sex, is_teacher, oauth_provider, oauth_foreign_id, salt
                     FROM session
                     WHERE managed_by = $1";
        group.members = self.query_map_many(query, &[&group_id], |row| SessionUser { id: row.get(0),
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
                                                                                     email_confirmationcode:
                                                                                         row.get(11),

                                                                                     firstname: row.get(12),
                                                                                     lastname: row.get(13),
                                                                                     street: row.get(14),
                                                                                     zip: row.get(15),
                                                                                     city: row.get(16),
                                                                                     nation: row.get(17),
                                                                                     grade: row.get(18),
                                                                                     sex: row.get(19),

                                                                                     is_teacher: row.get(20),
                                                                                     managed_by: Some(group_id),

                                                                                     oauth_provider: row.get(21),
                                                                                     oauth_foreign_id: row.get(22) })
                            .unwrap();
        Some(group)
    }

    fn get_debug_information(&self) -> String {
        let duration = Duration::minutes(60);
        let now = time::get_time();
        let then = now - duration;

        let query = "SELECT count(*)
                     FROM session
                     WHERE last_activity > $1;";
        let n_asession: i64 = self.query_map_one(query, &[&then], |row| row.get(0)).unwrap().unwrap();

        let query = "SELECT count(*)
                     FROM participation
                     WHERE start_date > $1;";
        let n_apart: i64 = self.query_map_one(query, &[&then], |row| row.get(0)).unwrap().unwrap();

        let query = "SELECT count(*)
                     FROM session;";
        let n_session: i64 = self.query_map_one(query, &[], |row| row.get(0)).unwrap().unwrap();

        let query = "SELECT count(*)
                     FROM session
                     WHERE oauth_foreign_id IS NOT NULL OR logincode IS NOT NULL;";
        let n_user: i64 = self.query_map_one(query, &[], |row| row.get(0)).unwrap().unwrap();

        let query = "SELECT count(*)
                     FROM session
                     WHERE oauth_foreign_id IS NOT NULL;";
        let n_pmsuser: i64 = self.query_map_one(query, &[], |row| row.get(0)).unwrap().unwrap();

        let query = "SELECT count(*)
                     FROM session
                     WHERE is_teacher = $1;";
        let n_teacher: i64 = self.query_map_one(query, &[&true], |row| row.get(0)).unwrap().unwrap();

        let query = "SELECT count(*)
                     FROM participation;";
        let n_part: i64 = self.query_map_one(query, &[], |row| row.get(0)).unwrap().unwrap();

        let query = "SELECT count(*)
                     FROM submission;";
        let n_sub: i64 = self.query_map_one(query, &[], |row| row.get(0)).unwrap().unwrap();

        format!(
                "{{
  'timestamp': {},
  'active_sessions': {},
  'active_participations': {},
  'sessions': {},
  'users': {},
  'pms_users': {},
  'teachers': {},
  'participations': {},
  'submissions': {}
}}
",
                now.sec, n_asession, n_apart, n_session, n_user, n_pmsuser, n_teacher, n_part, n_sub
        )
    }

    fn reset_all_contest_visibilities(&self) { self.execute("UPDATE contest SET public = $1", &[&false]).unwrap(); }
    fn reset_all_taskgroup_visibilities(&self) { self.execute("UPDATE taskgroup SET active = $1", &[&false]).unwrap(); }


    #[cfg(feature = "importforeign")]
    fn import_foreign_data(&self, infos: Vec<::foreigncontestimport::Info>) -> Result<(), ()> {
        for info in infos {
            let mut teacher_id: Option<i32> = None;
            let mut group_id: Option<i32> = None;

            if let Some(teacher) = info.teacher {
                let query = "SELECT id
                             FROM session
                             WHERE oauth_provider = $1
                             AND oauth_foreign_id = $2
                             LIMIT 1";

                teacher_id = Some(match self.query_map_one(query, &[&"pms", &teacher.pmsid], |row| row.get(0)).unwrap()
                                  {
                                      Some(id) => {
                                          let query = "UPDATE session
                                                       SET (permanent_login, grade, is_teacher, oauth_provider,
                                                            oauth_foreign_id, firstname, lastname)
                                                           = ($1, $2, $3, $4, $5, $6, $7)
                                                       WHERE id = $8";
                                          self.execute(query,
                                                       &[&true,
                                                         &255,
                                                         &true,
                                                         &"pms",
                                                         &teacher.pmsid,
                                                         &teacher.firstname,
                                                         &teacher.lastname,
                                                         &id])
                                              .unwrap();

                                          id
                                      },
                                      _ => {
                                          let csrf_token = helpers::make_csrf_token();
                                          let now = time::get_time();

                                          let query = "INSERT INTO session (session_token, csrf_token, last_activity,
                                                                            permanent_login, grade, is_teacher,
                                                                            oauth_provider, oauth_foreign_id, firstname,
                                                                            lastname)
                                                       VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)";
                                          self.execute(query,
                                                       &[&"",
                                                         &csrf_token,
                                                         &now,
                                                         &true,
                                                         &255,
                                                         &true,
                                                         &"pms",
                                                         &teacher.pmsid,
                                                         &teacher.firstname,
                                                         &teacher.lastname])
                                              .unwrap();

                                          self.get_last_id().expect("Expected to get last row id")
                                      }
                                  });
                println!("{:?}", teacher_id);
            }

            if let Some(group) = info.group {
                let query = "SELECT id
                             FROM usergroup
                             WHERE name = $1
                             AND admin = $2
                             LIMIT 1";

                let fallback_groupcode = helpers::make_group_code();

                group_id =
                    Some(match self.query_map_one(query, &[&group.groupname, &teacher_id.unwrap()], |row| row.get(0))
                                   .unwrap()
                         {
                             Some(id) => id,
                             _ => {
                                 let query = "INSERT INTO usergroup (name, groupcode, tag, admin)
                                     VALUES ($1, $2, $3, $4)";
                                 self.execute(query,
                                              &[&group.groupname,
                                                &group.groupcode.unwrap_or(fallback_groupcode),
                                                &group.groupname,
                                                &teacher_id.unwrap()])
                                     .unwrap();

                                 self.get_last_id().expect("Expected to get last row id")
                             }
                         });
                println!("{:?}", teacher_id);
            }

            let query = "SELECT id
                         FROM session
                         WHERE username = $1
                         OR logincode = $2
                         OR (oauth_foreign_id = $3 AND oauth_provider = $4)
                         LIMIT 1";

            let invalid = "@Qg9BpSC0qq:".to_string();

            let user = info.user;
            let user_id = match self.query_map_one(query,
                                                   &[user.username.as_ref().unwrap_or(&invalid),
                                                     user.logincode.as_ref().unwrap_or(&invalid),
                                                     user.pmsid.as_ref().unwrap_or(&invalid),
                                                     &"pms"],
                                                   |row| row.get(0))
                                    .unwrap()
            {
                Some(id) => {
                    let query = "UPDATE session
                                 SET (permanent_login, grade, is_teacher, oauth_provider, oauth_foreign_id, firstname,
                                      lastname, username, password, salt, street, zip, city, nation, managed_by)
                                     = ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
                                 WHERE id = $16";
                    self.execute(query,
                                 &[&true,
                                   &user.grade,
                                   &false,
                                   &user.pmsid.as_ref().map(|_| "pms"),
                                   &user.pmsid,
                                   &user.firstname,
                                   &user.lastname,
                                   &user.username,
                                   &user.password,
                                   &"",
                                   &user.street,
                                   &user.zip,
                                   &user.city,
                                   &user.nation,
                                   &group_id,
                                   &id])
                        .unwrap();

                    id
                },
                _ => {
                    let fallback_logincode = helpers::make_login_code();
                    let csrf_token = helpers::make_csrf_token();
                    let now = time::get_time();

                    let query = "INSERT INTO session (session_token, csrf_token, last_activity, permanent_login, grade,
                                                      is_teacher, oauth_provider, oauth_foreign_id, firstname, lastname,
                                                      logincode, username, password, salt, street, zip, city, nation,
                                                      managed_by)
                                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17,
                                             $18, $19)";
                    self.execute(query,
                                 &[&"",
                                   &csrf_token,
                                   &now,
                                   &true,
                                   &user.grade,
                                   &false,
                                   &user.pmsid.as_ref().map(|_| "pms"),
                                   &user.pmsid,
                                   &user.firstname,
                                   &user.lastname,
                                   &user.logincode.unwrap_or(fallback_logincode),
                                   &user.username,
                                   &user.password,
                                   &"",
                                   &user.street,
                                   &user.zip,
                                   &user.city,
                                   &user.nation,
                                   &group_id])
                        .unwrap();
                    self.get_last_id().expect("Expected to get last row id")
                }
            };
            println!("{:?}", user_id);
        }
        Ok(())
    }
}
