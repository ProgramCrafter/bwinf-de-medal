extern crate rusqlite;

use self::rusqlite::Connection;

use db_conn::{MedalConnection, MedalObject};
use db_objects::*;

use rand::{thread_rng, Rng};

fn hash_password(password: &str, hash: &str) -> String {
   password.to_string() 
}

impl MedalConnection for Connection {
    fn create() -> Connection {
        Connection::open("blub.db").unwrap()
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
        Some(self.query_row("SELECT id, csrf_token, last_login, last_activity, permanent_login, username, password, logincode, email, email_unconfirmed, email_confirmationcode, firstname, lastname, street, zip, city, nation, grade, is_teacher, managed_by, pms_id, pms_school_id FROM session_user WHERE session_token = ?1", &[&key], |row| {
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
        }).unwrap())
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
                if (hash_password(&password, &salt.unwrap()) == password_hash.unwrap()) {
                    // Login okay, update session now!
                    
                    let session_token: String = thread_rng().gen_ascii_chars().take(10).collect();
                    let csrf_token: String = thread_rng().gen_ascii_chars().take(10).collect();
                    let now = time::get_time();
                    
                    self.execute("UPDATE session_user SET session_token = ?1, csrf_token = ?2, last_login = ?3, last_activity = ?3 WHERE id = ?4", &[&session_token, &csrf_token, &now, &id]).unwrap();
                    
                    Ok(session_token)
                }
                else {println!("b");Err(()) }
                
            },
            _ => {println!("c"); Err(()) }
        }
        
    }
    fn login_with_code(&self, session: Option<String>, logincode: String) -> Result<SessionUser,()> {unimplemented!()}
    fn logout(&self, session: String) {
        self.execute("UPDATE session_user SET session_token = NULL WHERE id = ?1", &[&session]).unwrap();
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
        self.query_row(
            "SELECT task.location, task.stars, taskgroup.id, taskgroup.name, contest.id, contest.location, contest.filename, contest.name, contest.duration, contest.public, contest.start_date, contest.end_date FROM task JOIN taskgroup ON task.taskgroup = taskgroup.id JOIN contest ON taskgroup.contest = contest.id WHERE task.id = ?1",
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
}


impl MedalObject<Connection> for Task {
    fn save(&mut self, conn: &Connection) {
        conn.query_row("SELECT id FROM task WHERE taskgroup = ?1 AND location = ?2", &[&self.taskgroup, &self.location], |row| {row.get(0)})
            .and_then(|id| { self.setId(id); Ok(()) });
        
        let id = match self.getId() {
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
        self.setId(id);
    }
}


impl MedalObject<Connection> for Taskgroup {
    fn save(&mut self, conn: &Connection) {
        conn.query_row("SELECT id FROM taskgroup WHERE contest = ?1 AND name = ?2", &[&self.contest, &self.name], |row| {row.get(0)})
            .and_then(|id| { self.setId(id); Ok(()) });
        
        let id = match self.getId() {
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
        self.setId(id);
        for mut task in &mut self.tasks {
            task.taskgroup = id;
            task.save(conn);
        }
    }
}

impl MedalObject<Connection> for Contest {
    fn save(&mut self, conn: &Connection) {
        conn.query_row("SELECT id FROM contest WHERE location = ?1 AND filename = ?2", &[&self.location, &self.filename], |row| {row.get(0)})
            .and_then(|id| { self.setId(id); Ok(()) });
        
        let id = match self.getId() {
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
        self.setId(id);
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
        match self.getId() {
            Some(id) => 
                unimplemented!(),
            None => {
                conn.execute("INSERT INTO submission (task, session_user, grade, validated, nonvalidated_grade, subtask_identifier, value, date, needs_validation) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)", &[&self.task, &self.session_user, &self.grade, &self.validated, &self.nonvalidated_grade, &self.subtask_identifier, &self.value, &self.date, &self.needs_validation]).unwrap();
                // TODO, update ID
            }
        }
    }
}

impl MedalObject<Connection> for Group {
    fn save(&mut self, conn: &Connection) {
        match self.getId() {
            Some(id) => 
                unimplemented!(),
            None => {
                conn.execute("INSERT INTO usergroup (name, groupcode, tag, admin) VALUES (?1, ?2, ?3, ?4)", &[&self.name, &self.groupcode, &self.tag, &self.admin]).unwrap();
                // TODO, update ID
            }
        }
    }
}
