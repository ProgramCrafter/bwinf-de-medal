
extern crate time;

use self::time::{Timespec, Duration};

#[derive(Clone)]
pub struct SessionUser {
    pub id: u32,
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
    pub grade: u8,

    pub is_teacher: bool,
    pub managed_by: Option<u32>,
    pub pms_id: Option<u32>,
    pub pms_school_id: Option<u32>,
}

// Short version for display
#[derive(Clone, Default)]
pub struct UserInfo {
    pub id: u32,
    pub username: Option<String>,
    pub logincode: Option<String>,
    pub firstname: Option<String>,
    pub lastname: Option<String>,
}

#[derive(Clone)]
pub struct Group {
    pub id: Option<u32>,
    pub name: String,
    pub groupcode: String,
    pub tag: String,
    pub admin: u32,
    pub members: Vec<SessionUser>
}

pub struct Contest {
    pub id: Option<u32>,
    pub location: String,
    pub filename: String,
    pub name: String,
    pub duration: u32,
    pub public: bool,
    pub start: Option<Timespec>,
    pub end: Option<Timespec>,
    pub taskgroups: Vec<Taskgroup>,
}


pub struct Taskgroup {
    pub id: Option<u32>,
    pub contest: u32,
    pub name: String,
    pub tasks: Vec<Task>,
}

pub struct Task {
    pub id: Option<u32>,
    pub taskgroup: u32,
    pub location: String,
    pub stars: u8,
}


pub struct Submission {
    pub id: Option<u32>,
    pub session_user: u32,
    pub task: u32,
    pub grade: u32,
    pub validated: bool,
    pub nonvalidated_grade: u32,
    pub needs_validation: bool,
    pub subtask_identifier: Option<String>,
    pub value: String,
    pub date: Timespec,
}

#[derive(Clone, Copy, Default)]
pub struct Grade {
    pub taskgroup: u32,
    pub user: u32,
    pub grade: u8,
    pub validated: bool,
}

pub struct Participation {
    pub contest: u32,
    pub user: u32,
    pub start: Timespec,
}

pub trait HasId { fn get_id(&self) -> Option<u32>; fn set_id(&mut self, id: u32); }
impl HasId for Submission { fn get_id(&self) -> Option<u32> { self.id } fn set_id(&mut self, id: u32) { self.id = Some(id);} }
impl HasId for Task { fn get_id(&self) -> Option<u32> { self.id } fn set_id(&mut self, id: u32) { self.id = Some(id);} }
impl HasId for Taskgroup { fn get_id(&self) -> Option<u32> { self.id } fn set_id(&mut self, id: u32) { self.id = Some(id);} }
impl HasId for Contest { fn get_id(&self) -> Option<u32> { self.id } fn set_id(&mut self, id: u32) { self.id = Some(id);} }
impl HasId for Group { fn get_id(&self) -> Option<u32> { self.id } fn set_id(&mut self, id: u32) { self.id = Some(id);} }


impl Contest {
    pub fn new(location: String, filename: String, name: String, duration: u32, public: bool, start: Option<Timespec>, end: Option<Timespec>) -> Self {
        Contest {
            id: None,
            location: location,
            filename: filename,
            name: name,
            duration: duration,
            public: public,
            start: start,
            end: end,
            taskgroups: Vec::new(),
            
        }
    }    
}

impl SessionUser {
    pub fn minimal(id: u32, session_token: String, csrf_token: String) -> Self {
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
            
            is_teacher: false,
            managed_by: None,
            pms_id: None,
            pms_school_id: None,
        }
    }

    pub fn ensure_alive(self) -> Option<Self> {
        let duration = if self.permanent_login { Duration::days(90) } else { Duration::minutes(30) };
        let now = time::get_time();
        if now - self.last_activity? < duration {
            Some(self)
        }
        else {
            None
        }                                     
    }

    pub fn ensure_logged_in(self) -> Option<Self> {
        if self.password.is_some() || self.logincode.is_some() || self.pms_id.is_some() {
            self.ensure_alive()
        }                                     
        else {    
            None
        }
    }
}

impl Taskgroup {
    pub fn new(name: String) -> Self {
        Taskgroup {
            id: None,
            contest: 0,
            name: name,
            tasks: Vec::new(),
        }
    }    
}

impl Task {
    pub fn new(location: String, stars: u8) -> Self {
        Task {
            id: None,
            taskgroup: 0,
            location: location,
            stars: stars,
        }
    }    
}

