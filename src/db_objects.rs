extern crate time;

use self::time::{Duration, Timespec};

#[derive(Clone)]
pub struct SessionUser {
    pub id: i32,
    pub session_token: Option<String>, // delete this to log out
    pub csrf_token: String,
    pub last_login: Option<Timespec>,
    pub last_activity: Option<Timespec>,
    pub permanent_login: bool,

    pub username: Option<String>,
    pub password: Option<String>,
    pub salt: Option<String>,
    pub logincode: Option<String>,
    pub email: Option<String>,
    pub email_unconfirmed: Option<String>,
    pub email_confirmationcode: Option<String>,

    pub firstname: Option<String>,
    pub lastname: Option<String>,
    pub street: Option<String>,
    pub zip: Option<String>,
    pub city: Option<String>,
    pub nation: Option<String>,
    pub grade: i32,
    pub sex: Option<i32>,

    pub is_teacher: bool,
    pub managed_by: Option<i32>,

    pub oauth_foreign_id: Option<String>,
    pub oauth_provider: Option<String>,
    // pub oauth_extra_data: Option<String>,

    // pub pms_id: Option<i32>,
    // pub pms_school_id: Option<i32>,
}

// Short version for display
#[derive(Clone, Default)]
pub struct UserInfo {
    pub id: i32,
    pub username: Option<String>,
    pub logincode: Option<String>,
    pub firstname: Option<String>,
    pub lastname: Option<String>,
    pub grade: i32,
}

#[derive(Clone)]
pub struct Group {
    pub id: Option<i32>,
    pub name: String,
    pub groupcode: String,
    pub tag: String,
    pub admin: i32,
    pub members: Vec<SessionUser>,
}

#[derive(Debug)]
pub struct Contest {
    pub id: Option<i32>,
    pub location: String,
    pub filename: String,
    pub name: String,
    pub duration: i32,
    pub public: bool,
    pub start: Option<Timespec>,
    pub end: Option<Timespec>,
    pub min_grade: Option<i32>,
    pub max_grade: Option<i32>,
    pub positionalnumber: Option<i32>,
    pub requires_login: Option<bool>,
    pub secret: Option<String>,
    pub taskgroups: Vec<Taskgroup>,
}

#[derive(Debug)]
pub struct Taskgroup {
    pub id: Option<i32>,
    pub contest: i32,
    pub name: String,
    pub active: bool,
    pub positionalnumber: Option<i32>,
    pub tasks: Vec<Task>,
}

#[derive(Debug)]
pub struct Task {
    pub id: Option<i32>,
    pub taskgroup: i32,
    pub location: String,
    pub stars: i32,
}

pub struct Submission {
    pub id: Option<i32>,
    pub session_user: i32,
    pub task: i32,
    pub grade: i32,
    pub validated: bool,
    pub nonvalidated_grade: i32,
    pub needs_validation: bool,
    pub subtask_identifier: Option<String>,
    pub value: String,
    pub date: Timespec,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct Grade {
    pub taskgroup: i32,
    pub user: i32,
    pub grade: Option<i32>,
    pub validated: bool,
}

pub struct Participation {
    pub contest: i32,
    pub user: i32,
    pub start: Timespec,
}

pub trait HasId {
    fn get_id(&self) -> Option<i32>;
    fn set_id(&mut self, id: i32);
}
impl HasId for Submission {
    fn get_id(&self) -> Option<i32> { self.id }
    fn set_id(&mut self, id: i32) { self.id = Some(id); }
}
impl HasId for Task {
    fn get_id(&self) -> Option<i32> { self.id }
    fn set_id(&mut self, id: i32) { self.id = Some(id); }
}
impl HasId for Taskgroup {
    fn get_id(&self) -> Option<i32> { self.id }
    fn set_id(&mut self, id: i32) { self.id = Some(id); }
}
impl HasId for Contest {
    fn get_id(&self) -> Option<i32> { self.id }
    fn set_id(&mut self, id: i32) { self.id = Some(id); }
}
impl HasId for Group {
    fn get_id(&self) -> Option<i32> { self.id }
    fn set_id(&mut self, id: i32) { self.id = Some(id); }
}

impl Contest {
    // TODO: Rewrite, so this attribute can be removed
    #[allow(clippy::too_many_arguments)]
    pub fn new(location: String, filename: String, name: String, duration: i32, public: bool,
               start: Option<Timespec>, end: Option<Timespec>, min_grade: Option<i32>, max_grade: Option<i32>,
               positionalnumber: Option<i32>, requires_login: Option<bool>, secret: Option<String>)
               -> Self
    {
        Contest { id: None,
                  location: location,
                  filename: filename,
                  name: name,
                  duration: duration,
                  public: public,
                  start: start,
                  end: end,
                  min_grade: min_grade,
                  max_grade: max_grade,
                  positionalnumber: positionalnumber,
                  requires_login: requires_login,
                  secret: secret,
                  taskgroups: Vec::new() }
    }
}

impl SessionUser {
    pub fn minimal(id: i32, session_token: String, csrf_token: String) -> Self {
        SessionUser {
            id: id,
            session_token: Some(session_token),
            csrf_token: csrf_token,
            last_login: None,
            last_activity: None, // now?
            // müssen die überhaupt außerhalb der datenbankabstraktion sichtbar sein?
            permanent_login: false,

            username: None,
            password: None,
            salt: None,
            logincode: None,
            email: None,
            email_unconfirmed: None,
            email_confirmationcode: None,

            firstname: None,
            lastname: None,
            street: None,
            zip: None,
            city: None,
            nation: None,
            grade: 0,
            sex: None,

            is_teacher: false,
            managed_by: None,

            oauth_foreign_id: None,
            oauth_provider: None,
            // oauth_extra_data: Option<String>,

            //pms_id: None,
            //pms_school_id: None,
        }
    }

    pub fn group_user_stub() -> Self {
        SessionUser { id: 0,
                      session_token: None,
                      csrf_token: "".to_string(),
                      last_login: None,
                      last_activity: None,
                      permanent_login: false,

                      username: None,
                      password: None,
                      salt: None,
                      logincode: None,
                      email: None,
                      email_unconfirmed: None,
                      email_confirmationcode: None,

                      firstname: None,
                      lastname: None,
                      street: None,
                      zip: None,
                      city: None,
                      nation: None,
                      grade: 0,
                      sex: None,

                      is_teacher: false,
                      managed_by: None,

                      oauth_foreign_id: None,
                      oauth_provider: None }
    }

    pub fn is_alive(&self) -> bool {
        let duration = if self.permanent_login { Duration::days(90) } else { Duration::hours(9) };
        let now = time::get_time();
        if let Some(last_activity) = self.last_activity {
            now - last_activity < duration
        } else {
            false
        }
    }

    pub fn is_logged_in(&self) -> bool {
        (self.password.is_some() || self.logincode.is_some() || self.oauth_foreign_id.is_some()) && self.is_alive()
    }

    pub fn is_admin(&self) -> bool {
        self.id == 1
    }

    pub fn ensure_alive(self) -> Option<Self> {
        if self.is_alive() {
            Some(self)
        } else {
            None
        }
    }

    pub fn ensure_logged_in(self) -> Option<Self> {
        if self.is_logged_in() {
            Some(self)
        } else {
            None
        }
    }
}

impl Taskgroup {
    pub fn new(name: String, positionalnumber: Option<i32>) -> Self {
        Taskgroup { id: None,
                    contest: 0,
                    name: name,
                    active: true,
                    positionalnumber: positionalnumber,
                    tasks: Vec::new() }
    }
}

impl Task {
    pub fn new(location: String, stars: i32) -> Self {
        Task { id: None, taskgroup: 0, location: location, stars: stars }
    }
}

pub trait OptionSession {
    fn ensure_alive(self) -> Self;
    fn ensure_logged_in(self) -> Self;
}

impl OptionSession for Option<SessionUser> {
    fn ensure_alive(self) -> Self { self?.ensure_alive() }
    fn ensure_logged_in(self) -> Self { self?.ensure_logged_in() }
}
