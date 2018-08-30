extern crate rusqlite;

use self::rusqlite::Connection;

use db_conn::{MedalConnection, MedalObject};
use db_objects::*;

use rand::{thread_rng, Rng, distributions::Alphanumeric};

use time;

use std::path::{Path};

use ::functions; // todo: remove (usertype in db)


fn hash_password(password: &str, hash: &str) -> String {
   password.to_string() 
}

impl MedalConnection for Connection {
    fn create(file: &Path) -> Connection {
        Connection::open(file).unwrap()
    }

    fn dbtype(&self) -> &'static str {
        return "sqlite";
    }

    fn migration_already_applied(&self, name: &str) -> bool {
        let create_string = "CREATE TABLE IF NOT EXISTS migrations (name TEXT PRIMARY KEY);";
        self.execute(create_string, &[]).unwrap();
        
        let mut stmt = self.prepare("SELECT name FROM migrations WHERE name = ?1").unwrap();
        stmt.exists(&[&name]).unwrap()
    }
    
    fn apply_migration(&mut self, name: &str, contents: String) {
        print!("Applying migration `{}` … ", name);
        
        let tx = self.transaction().unwrap();
        
        tx.execute(&contents, &[]).unwrap();
        tx.execute("INSERT INTO migrations (name) VALUES (?1)", &[&name]).unwrap();
        
        tx.commit().unwrap();
        
        println!("OK.");
    }

    fn get_session(&self, key: String) -> Option<SessionUser> {
        let res = self.query_row("SELECT id, csrf_token, last_login, last_activity, permanent_login, username, password, logincode, email, email_unconfirmed, email_confirmationcode, firstname, lastname, street, zip, city, nation, grade, is_teacher, managed_by, pms_id, pms_school_id FROM session_user WHERE session_token = ?1", &[&key], |row| {
            SessionUser {
                id: row.get(0),
                session_token: Some(key.clone()),
                csrf_token: row.get(1),
                last_login: row.get(2),
                last_activity: row.get(3),
                permanent_login: row.get(4),
                
                username: row.get(5),
                password: row.get(6),
                salt: None,//"".to_string(),
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
                pms_id: row.get(20),
                pms_school_id: row.get(21),
            }
        });
        match res {
            Ok(session) => Some(session),
            _ => None
        }
    }
    fn save_session(&self, session: SessionUser) {
        self.execute("UPDATE session_user SET
                      username = ?1,
                      password = ?2,
                      logincode = ?3,
                      firstname = ?4,
                      lastname = ?5,
                      grade = ?6 WHERE id = ?", &[&session.username, &session.password, &session.logincode, &session.firstname, &session.lastname, &session.grade, &session.id]).unwrap();
    }
    fn new_session(&self) -> SessionUser {
        let session_token = "123".to_string();
        let csrf_token = "123".to_string();
        
        self.execute("INSERT INTO session_user (session_token, csrf_token)
                      VALUES (?1, ?2)",
            &[&session_token, &csrf_token]).unwrap();
        let id = self.query_row("SELECT last_insert_rowid()", &[], |row| {row.get(0)}).unwrap();
        
        SessionUser::minimal(id, session_token, csrf_token)
    }
    fn get_session_or_new(&self, key: String) -> SessionUser {
        self.get_session(key).unwrap_or_else(|| self.new_session())
    }

    fn login(&self, session: Option<String>, username: String, password: String) -> Result<String,()> {
        println!("a {} {}", username, password);
        match self.query_row(
            "SELECT id, password, salt FROM session_user WHERE username = ?1",
            &[&username],
            |row| -> (u32, Option<String>, Option<String>) {
                (row.get(0), row.get(1), row.get(2))    
            }) {
            Ok((id, password_hash, salt)) => {
                //println!("{}, {}", password, password_hash.unwrap());
                if hash_password(&password, &salt.unwrap()) == password_hash.unwrap() {
                    // Login okay, update session now!
                    
                    let session_token: String = thread_rng().sample_iter(&Alphanumeric).take(10).collect();
                    let csrf_token: String = thread_rng().sample_iter(&Alphanumeric).take(10).collect();
                    let now = time::get_time();
                    
                    self.execute("UPDATE session_user SET session_token = ?1, csrf_token = ?2, last_login = ?3, last_activity = ?3 WHERE id = ?4", &[&session_token, &csrf_token, &now, &id]).unwrap();
                    
                    Ok(session_token)
                }
                else {println!("b");Err(()) }
                
            },
            _ => {println!("c"); Err(()) }
        }
    }   
    fn login_with_code(&self, session: Option<String>, logincode: String) -> Result<String,()> {
        println!("a {}", logincode);
        match self.query_row(
            "SELECT id FROM session_user WHERE logincode = ?1",
            &[&logincode],
            |row| -> u32 {
                row.get(0)    
            }) {
            Ok(id) => {
                // Login okay, update session now!
                    
                let session_token: String = thread_rng().sample_iter(&Alphanumeric).take(10).collect();
                let csrf_token: String = thread_rng().sample_iter(&Alphanumeric).take(10).collect();
                let now = time::get_time();
                    
                self.execute("UPDATE session_user SET session_token = ?1, csrf_token = ?2, last_login = ?3, last_activity = ?3 WHERE id = ?4", &[&session_token, &csrf_token, &now, &id]).unwrap();
                    
                Ok(session_token)
            },
            _ => {println!("c"); Err(()) }
        }
    }
    
    fn login_foreign(&self, session: Option<String>, foreign_id: u32, foreign_type: functions::UserType, firstname: String, lastname:String) -> Result<String,()> {
        let session_token: String = thread_rng().sample_iter(&Alphanumeric).take(10).collect();
        let csrf_token: String = thread_rng().sample_iter(&Alphanumeric).take(10).collect();
        let now = time::get_time();
        
        println!("x {} {}", firstname, lastname);
        match self.query_row(
            "SELECT id FROM session_user WHERE pms_id = ?1",
            &[&foreign_id],
            |row| -> u32 {row.get(0)}) {
            Ok(id) => {                    
                self.execute("UPDATE session_user SET session_token = ?1, csrf_token = ?2, last_login = ?3, last_activity = ?3 WHERE id = ?4", &[&session_token, &csrf_token, &now, &id]).unwrap();
                
                Ok(session_token)              
            },
            // Add!
            _ => {
                self.execute("INSERT INTO session_user (session_token, csrf_token, last_login, last_activity, permanent_login, grade, is_teacher, pms_id, firstname, lastname) VALUES (?1, ?2, ?3, ?3, ?4, ?5, ?6, ?7, ?8, ?9)", &[&session_token, &csrf_token, &now, &false, &0, &(foreign_type != functions::UserType::User), &foreign_id, &firstname, &lastname]).unwrap();
                
                Ok(session_token)
            }
        }
    }

    fn create_user_with_groupcode(&self, session: Option<String>, groupcode: String) -> Result<String,()> {
        println!("a {}", groupcode);
        match self.query_row(
            "SELECT id FROM usergroup WHERE groupcode = ?1",
            &[&groupcode],
            |row| -> u32 {
                row.get(0)    
            }) {
            Ok(group_id) => {
                // Login okay, create session_user!
                    
                let session_token: String = thread_rng().sample_iter(&Alphanumeric).take(10).collect();
                let csrf_token: String = thread_rng().sample_iter(&Alphanumeric).take(10).collect();
                let login_code: String = Some('u').into_iter().chain(thread_rng().sample_iter(&Alphanumeric))
                    .filter(|x| {let x = *x; !(x == 'l' || x == 'I' || x == '1' || x == 'O' || x == 'o' || x == '0')})
                    .take(9).collect();
                // todo: check for collisions
                let now = time::get_time();
                    
                self.execute("INSERT INTO session_user (session_token, csrf_token, last_login, last_activity, permanent_login, logincode, grade, is_teacher, managed_by) VALUES (?1, ?2, ?3, ?3, ?4, ?5, ?6, ?7, ?8)", &[&session_token, &csrf_token, &now, &false, &login_code, &0, &false, &group_id]).unwrap();
                    
                Ok(session_token)
            },
            _ => {println!("c"); Err(()) }
        }
    }
    
    fn logout(&self, session: String) {
        self.execute("UPDATE session_user SET session_token = NULL WHERE session_token = ?1", &[&session]).unwrap();
    }

    
    fn load_submission(&self, session: &SessionUser, task: u32, subtask: Option<String>) -> Option<Submission> {
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
                    subtask_identifier: Some(subtask_id.clone()),
                    value: row.get(4),
                    date: row.get(5),
                    needs_validation: row.get(6),
                }
            }).ok()
        }
    }
    fn submit_submission(&self, mut submission: Submission) {
        submission.save(self);
    }

    fn get_contest_list(&self) -> Vec<Contest> {
        let mut stmt = self.prepare("SELECT id, location, filename, name, duration, public, start_date, end_date FROM contest").unwrap();
        let rows = stmt.query_map(&[], |row| {
            Contest {
                id: Some(row.get(0)),
                location: row.get(1),
                filename: row.get(2),
                name: row.get(3),
                duration: row.get(4),
                public: row.get(5),
                start: row.get(6),
                end: row.get(7),
                taskgroups: Vec::new(),            
            }
        }).unwrap().filter_map(|row| {row.ok()}).collect();
        rows
    }
    
    fn get_contest_by_id(&self, contest_id : u32) -> Contest {
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
    
    fn get_contest_by_id_complete(&self, contest_id : u32) -> Contest {
        let mut stmt = self.prepare("SELECT contest.location, contest.filename, contest.name, contest.duration, contest.public, contest.start_date, contest.end_date, taskgroup.id, taskgroup.name, task.id, task.location, task.stars FROM contest JOIN taskgroup ON contest.id = taskgroup.contest JOIN task ON taskgroup.id = task.taskgroup WHERE contest.id = ?1").unwrap();
        
        let mut taskgroupcontest_iter = stmt.query_map(&[&contest_id], |row| {
            (Contest {
                id: Some(contest_id),
                location: row.get(0),
                filename: row.get(1),
                name: row.get(2),
                duration: row.get(3),
                public: row.get(4),
                start: row.get(5),
                end: row.get(6),
                taskgroups: Vec::new(),            
            },Taskgroup {
                id: Some(row.get(7)),
                contest: contest_id,
                name: row.get(8),
                tasks: Vec::new(),  
            },Task {
                id: Some(row.get(9)),
                taskgroup: row.get(7),
                location: row.get(10),
                stars: row.get(11),
            })
        }).unwrap();

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
    fn get_participation(&self, session: String, contest_id: u32) -> Option<Participation> {
        self.query_row("SELECT user, start_date FROM participation JOIN session_user ON session_user.id = user WHERE session_user.session_token = ?1 AND contest = ?2", &[&session, &contest_id], |row| {
            Participation {
                contest: contest_id,
                user: row.get(0),
                start: row.get(1)
            }
        }).ok()
    }
    fn new_participation(&self, session: String, contest_id: u32) -> Result<Participation, ()> {
        match self.query_row("SELECT user, start_date FROM participation JOIN session_user ON session_user.id = user WHERE session_user.session_token = ?1 AND contest = ?2", &[&session, &contest_id], |row| {()}) {
            Ok(()) => Err(()),
            Err(_) => {
                let now = time::get_time();
                self.execute(
                    "INSERT INTO participation (contest, user, start_date)
                     SELECT ?1, id, ?2 FROM session_user WHERE session_token = ?3",
                     &[&contest_id, &now, &session]).unwrap();

                Ok(self.get_participation(session, contest_id).unwrap())
            }
        }
        
    }
    fn get_task_by_id(&self, task_id : u32) -> Task {
        self.query_row(
            "SELECT location, stars, taskgroup FROM task WHERE id = ?1",
            &[&task_id],
            |row| {
                Task {
                    id: Some(task_id),
                    taskgroup: row.get(2),
                    location: row.get(0),
                    stars: row.get(1)
                }
            }).unwrap()
    }
    fn get_task_by_id_complete(&self, task_id : u32) -> (Task, Taskgroup, Contest) {
        println!("{}!!", task_id);
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
    
    fn get_submission_to_validate(&self, tasklocation: String, subtask: Option<String>) -> u32{
        match subtask {
            Some(st) => self.query_row("SELECT id FROM submission JOIN task ON submission.task = task.id WHERE task.location = ?1  AND subtask_identifier = ?2 AND needs_validation = 1 LIMIT 1", &[&tasklocation, &st], |row| {row.get(0)}).unwrap(),
            None => self.query_row("SELECT id FROM submission JOIN task ON submission.task = task.id WHERE task.location = ?1 AND needs_validation = 1 LIMIT 1", &[&tasklocation], |row| {row.get(0)}).unwrap(),
        }
    }

    fn find_next_submission_to_validate(&self, userid: u32, taskgroupid: u32) {
        let (id, validated) : (u32, bool) = self.query_row("SELECT id, validated FROM submission JOIN task ON submission.task = task.id WHERE task.taskgroup = ?1 AND submission.user = ?2 ORDER BY value DESC id DESC LIMIT 1", &[&taskgroupid, &userid], |row| {(row.get(0), row.get(1))}).unwrap();;
        if !validated {
            self.execute("UPDATE submission SET needs_validation = 1 WHERE id = ?1", &[&id]).unwrap();
        }
    }


    fn add_group(&self, group: &mut Group) {
        group.save(self);
    }

    fn get_groups(&self, session_id: u32) -> Vec<Group> {
        let mut stmt = self.prepare("SELECT id, name, groupcode, tag FROM usergroup WHERE admin = ?1").unwrap();
        let rows = stmt.query_map(&[&session_id], |row| {
            Group {
                id: Some(row.get(0)),
                name: row.get(1),
                groupcode: row.get(2),
                tag: row.get(3),
                admin: session_id,
                members: Vec::new(),
            }
        }).unwrap().filter_map(|row| {row.ok()}).collect();
        rows
    }
    fn get_groups_complete(&self, session_id: u32) -> Vec<Group> {unimplemented!();}
    fn get_group_complete(&self, group_id: u32) -> Option<Group> {
        let mut group = self.query_row("SELECT name, groupcode, tag, admin FROM usergroup WHERE id  = ?1", &[&group_id], |row| {
            Group {
                id: Some(group_id),
                name: row.get(0),
                groupcode: row.get(1),
                tag: row.get(2),
                admin: row.get(3),
                members: Vec::new(),
            }
        }).unwrap(); // TODO handle error

        let mut stmt = self.prepare("SELECT id, session_token, csrf_token, last_login, last_activity, permanent_login, username, password, logincode, email, email_unconfirmed, email_confirmationcode, firstname, lastname, street, zip, city, nation, grade, is_teacher, pms_id, pms_school_id FROM session_user WHERE managed_by = ?1").unwrap();
        let rows = stmt.query_map(&[&group_id], |row| {
            SessionUser {
                id: row.get(0),
                session_token: row.get(1),
                csrf_token: row.get(2),
                last_login: row.get(3),
                last_activity: row.get(4),
                permanent_login: row.get(5),
                
                username: row.get(6),
                password: row.get(7),
                salt: None,//"".to_string(),
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
                pms_id: row.get(20),
                pms_school_id: row.get(21),
            }
        }).unwrap();

        for user in rows {
            group.members.push(user.unwrap());
        }
        Some(group)
    }
}


impl MedalObject<Connection> for Task {
    fn save(&mut self, conn: &Connection) {
        conn.query_row("SELECT id FROM task WHERE taskgroup = ?1 AND location = ?2", &[&self.taskgroup, &self.location], |row| {row.get(0)})
            .and_then(|id| { self.set_id(id); Ok(()) }).unwrap_or(()); // Err means no entry yet and is expected result
        
        let id = match self.get_id() {
            Some(id) => {
                conn.execute(
                    "UPDATE task SET taskgroup = ?1, location = ?2, stars = ?3
                     WHERE id = ?4",
                    &[&self.taskgroup, &self.location, &self.stars, &id]).unwrap();
                id
            }
            None => {                
                conn.execute(
                    "INSERT INTO task (taskgroup, location, stars)
                     VALUES (?1, ?2, ?3)",
                    &[&self.taskgroup, &self.location, &self.stars]).unwrap();
                conn.query_row("SELECT last_insert_rowid()", &[], |row| {row.get(0)}).unwrap()
            }
        };
        self.set_id(id);
    }
}


impl MedalObject<Connection> for Taskgroup {
    fn save(&mut self, conn: &Connection) {
        conn.query_row("SELECT id FROM taskgroup WHERE contest = ?1 AND name = ?2", &[&self.contest, &self.name], |row| {row.get(0)})
            .and_then(|id| { self.set_id(id); Ok(()) }).unwrap_or(()); // Err means no entry yet and is expected result
        
        let id = match self.get_id() {
            Some(id) => {
                conn.execute(
                    "UPDATE taskgroup SET contest = ?1, name = ?2
                     WHERE id = ?3",
                    &[&self.contest, &self.name, &id]).unwrap();
                id
            }
            None => {                
                conn.execute(
                    "INSERT INTO taskgroup (contest, name)
                     VALUES (?1, ?2)",
                    &[&self.contest, &self.name]).unwrap();
                conn.query_row("SELECT last_insert_rowid()", &[], |row| {row.get(0)}).unwrap()
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
        conn.query_row("SELECT id FROM contest WHERE location = ?1 AND filename = ?2", &[&self.location, &self.filename], |row| {row.get(0)})
            .and_then(|id| { self.set_id(id); Ok(()) }).unwrap_or(()); // Err means no entry yet and is expected result
        
        let id = match self.get_id() {
            Some(id) => {
                conn.execute(
                    "UPDATE contest SET location = ?1,filename = ?2,
                     name = ?3, duration = ?4, public = ?5, start_date = ?6,
                     end_date = ?7 WHERE id = ?8",
                    &[&self.location, &self.filename, &self.name,
                      &self.duration, &self.public, &self.start, &self.end,
                      &id]).unwrap();
                id
            }
            None => {                
                conn.execute(
                    "INSERT INTO contest (location, filename, name, duration, public, start_date, end_date)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    &[&self.location, &self.filename, &self.name,
                      &self.duration, &self.public, &self.start, &self.end]).unwrap();
                conn.query_row("SELECT last_insert_rowid()", &[], |row| {row.get(0)}).unwrap()
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
        conn.execute("INSERT OR REPLACE INTO grade (taskgroup, user, grade, validated) VALUES (?1, ?2, ?3, ?4)", &[&self.taskgroup, &self.user, &self.grade, &self.validated]).unwrap();
    }
}

impl MedalObject<Connection> for Participation {
    fn save(&mut self, conn: &Connection) {
        conn.execute("INSERT INTO participation (contest, user, start_date) VALUES (?1, ?2, ?3)", &[&self.contest, &self.user, &self.start]).unwrap();
    }
}


impl MedalObject<Connection> for Submission {
    fn save(&mut self, conn: &Connection) {
        match self.get_id() {
            Some(_id) => 
                unimplemented!(),
            None => {
                conn.execute("INSERT INTO submission (task, session_user, grade, validated, nonvalidated_grade, subtask_identifier, value, date, needs_validation) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)", &[&self.task, &self.session_user, &self.grade, &self.validated, &self.nonvalidated_grade, &self.subtask_identifier, &self.value, &self.date, &self.needs_validation]).unwrap();
                self.set_id(conn.query_row("SELECT last_insert_rowid()", &[], |row| {row.get(0)}).unwrap())
            }
        }
    }
}

impl MedalObject<Connection> for Group {
    fn save(&mut self, conn: &Connection) {
        match self.get_id() {
            Some(_id) => 
                unimplemented!(),
            None => {
                conn.execute("INSERT INTO usergroup (name, groupcode, tag, admin) VALUES (?1, ?2, ?3, ?4)", &[&self.name, &self.groupcode, &self.tag, &self.admin]).unwrap();
                self.set_id(conn.query_row("SELECT last_insert_rowid()", &[], |row| {row.get(0)}).unwrap());
            }
        }
    }
}
