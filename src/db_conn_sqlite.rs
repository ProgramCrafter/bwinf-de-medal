extern crate rusqlite;

use self::rusqlite::Connection;

use db_conn::{MedalConnection, MedalObject};
use db_objects::*;

impl MedalConnection for Connection {
    fn create() -> Connection {
        Connection::open("blub.db").unwrap()
    }

    fn dbtype(&self) -> &'static str {
        return "sqlite";
    }

    fn migration_already_applied(&mut self, name: &str) -> bool {
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

    fn get_session(&mut self, key: String) -> Option<SessionUser> {
        self.query_row("SELECT id, session_token, csrf_token, last_login, last_activity, permanent_login, username, password, logincode, email, email_unconfirmed, email_confirmation_code, firstname, lastname, street, zip, city, nation, grade, is_teacher, managed_by, pms_id, pms_school_id FROM session_user WHERE session_token = ?1", &[&key], |row| {
            SessionUser {
                id: row.get(0),
                session_token: Some(key.clone()),
                csrf_token: row.get(1),
                last_login: row.get(2),
                last_activity: row.get(3),
                permanent_login: row.get(4),
                
                username: row.get(5),
                password: row.get(6),
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
        }).ok()
    }
    fn new_session(&mut self) -> SessionUser {
        let session_token = "123".to_string();
        let csrf_token = "123".to_string();
        
        self.execute("INSERT INTO session_user (session_token, csrf_token)
                      VALUES (?1, ?2)",
            &[&session_token, &csrf_token]).unwrap();
        let id = self.query_row("SELECT last_insert_rowid()", &[], |row| {row.get(0)}).unwrap();
        
        SessionUser::minimal(id, session_token, csrf_token)
    }
    fn get_session_or_new(&mut self, key: String) -> SessionUser {
        self.get_session(key).unwrap_or_else(|| self.new_session())
    }

    fn login(&mut self, session: &SessionUser, username: String, password: String) -> Result<SessionUser,()> {unimplemented!()}
    fn login_with_code(&mut self, session: &SessionUser, logincode: String) -> Result<SessionUser,()> {unimplemented!()}
    fn logout(&mut self, session: &SessionUser) {
        self.execute("UPDATE session_user SET session_token = NULL WHERE id = ?1", &[&session.id]).unwrap();
    }

    
    fn load_submission(&mut self, session: &SessionUser, task: String, subtask: Option<String>) -> Submission {unimplemented!()}
    fn submit_submission(&mut self, session: &SessionUser, task: String, subtask: Option<String>, submission: Submission) {unimplemented!()}

    fn get_contest_by_id(&mut self, contest_id : u32) -> Contest {
        self.query_row("SELECT location, filename, name, duration, public, start, end FROM contest WHERE id = ?1", &[&contest_id], |row| {
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
    fn get_contest_by_id_complete(&mut self, contest_id : u32) -> Contest {
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
    fn get_task_by_id(&mut self, task_id : u32) -> Task {
        self.query_row(
            "SELECT location, stars, taskgroup WHERE id = ?1",
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
    fn get_task_by_id_complete(&mut self, task_id : u32) -> (Task, Taskgroup, Contest) {
        self.query_row(
            "SELECT task.location, task.stars, taskgroup.id, taskgroup.name, contest.id, contest.location, contest.filename, contest.name, contest.duration, contest.public, contest.start_date, contest.end_date JOIN taskgroup ON taskgroup = taskgroup.id JOIN contest ON taskgroup.contest = contest.id WHERE task.id = ?1",
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
    
    fn get_submission_to_validate(&mut self, tasklocation: String, subtask: Option<String>) -> u32{
        match subtask {
            Some(st) => self.query_row("SELECT id FROM submission JOIN task ON submission.task = task.id WHERE task.location = ?1  AND subtask_identifier = ?2 AND needs_validation = 1 LIMIT 1", &[&tasklocation, &st], |row| {row.get(0)}).unwrap(),
            None => self.query_row("SELECT id FROM submission JOIN task ON submission.task = task.id WHERE task.location = ?1 AND needs_validation = 1 LIMIT 1", &[&tasklocation], |row| {row.get(0)}).unwrap(),
        }
    }

    fn find_next_submission_to_validate(&mut self, userid: u32, taskgroupid: u32) {
        let (id, validated) : (u32, bool) = self.query_row("SELECT id, validated FROM submission JOIN task ON submission.task = task.id WHERE task.taskgroup = ?1 AND submission.user = ?2 ORDER BY value DESC id DESC LIMIT 1", &[&taskgroupid, &userid], |row| {(row.get(0), row.get(1))}).unwrap();;
        if !validated {
            self.execute("UPDATE submission SET needs_validation = 1 WHERE id = ?1", &[&id]).unwrap();
        }
    }
}


impl MedalObject<Connection> for Task {
    fn save(&mut self, conn: &mut Connection) {
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
    fn save(&mut self, conn: &mut Connection) {
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
    fn save(&mut self, conn: &mut Connection) {
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
    fn save(&mut self, conn: &mut Connection) {
        conn.execute("INSERT OR REPLACE INTO grade (taskgroup, user, grade, validated) VALUES (?1, ?2, ?3, ?4)", &[&self.taskgroup, &self.user, &self.grade, &self.validated]).unwrap();
    }
}

impl MedalObject<Connection> for Participation {
    fn save(&mut self, conn: &mut Connection) {
        conn.execute("INSERT INTO participation (contest, user, start_date) VALUES (?1, ?2, ?3)", &[&self.contest, &self.user, &self.start]).unwrap();
    }
}

