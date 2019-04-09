extern crate bcrypt;

use webfw_iron::{json_val, to_json};

use time;

use rand::{distributions::Alphanumeric, thread_rng, Rng};

use db_conn::MedalConnection;

use db_objects::{Group, SessionUser, Submission};

use self::bcrypt::hash;

#[derive(Serialize, Deserialize)]
pub struct SubTaskInfo {
    pub id: u32,
    pub linktext: String,
}

#[derive(Serialize, Deserialize)]
pub struct TaskInfo {
    pub name: String,
    pub subtasks: Vec<SubTaskInfo>,
}

#[derive(Serialize, Deserialize)]
pub struct ContestInfo {
    pub id: u32,
    pub location: String,
    pub filename: String,
    pub name: String,
    pub duration: u32,
    pub public: bool,
    pub tasks: Vec<TaskInfo>,
}

#[allow(dead_code)] //CsrfCheckFailed, SessionTimeout, DatabaseError and UnmatchedPasswords are used in webfw_iron.rs.
#[derive(Clone)]
pub enum MedalError {
    NotLoggedIn,
    AccessDenied,
    CsrfCheckFailed,
    SessionTimeout,
    DatabaseError,
    PasswordHashingError,
    UnmatchedPasswords,
}

type MedalValue = (String, json_val::Map<String, json_val::Value>);
type MedalResult<T> = Result<T, MedalError>;
type MedalValueResult = MedalResult<MedalValue>;

pub fn index<T: MedalConnection>(conn: &T, session_token: Option<String>,
                                 (self_url, oauth_url): (Option<String>, Option<String>))
                                 -> (String, json_val::Map<String, json_val::Value>)
{
    let mut data = json_val::Map::new();

    //let mut contests = Vec::new();

    if let Some(token) = session_token {
        if let Some(session) = conn.get_session(&token) {
            data.insert("logged_in".to_string(), to_json(&true));
            data.insert("username".to_string(), to_json(&session.username));
            data.insert("firstname".to_string(), to_json(&session.firstname));
            data.insert("lastname".to_string(), to_json(&session.lastname));
            data.insert("teacher".to_string(), to_json(&session.is_teacher));
        }
    }

    data.insert("self_url".to_string(), to_json(&self_url));
    data.insert("oauth_url".to_string(), to_json(&oauth_url));
    /*contests.push("blaa".to_string());
    data.insert("contest".to_string(), to_json(&contests));*/

    ("index".to_owned(), data)
}

pub fn show_contests<T: MedalConnection>(conn: &T) -> MedalValue {
    let mut data = json_val::Map::new();

    let v: Vec<ContestInfo> = conn.get_contest_list()
                                  .iter()
                                  .map(|c| ContestInfo { id: c.id.unwrap(),
                                                         location: c.location.clone(),
                                                         filename: c.filename.clone(),
                                                         name: c.name.clone(),
                                                         duration: c.duration,
                                                         public: c.public,
                                                         tasks: Vec::new() })
                                  .collect();
    data.insert("contest".to_string(), to_json(&v));

    ("contests".to_owned(), data)
}

pub fn show_contest<T: MedalConnection>(conn: &T, contest_id: u32, session_token: String) -> MedalValueResult {
    let c = conn.get_contest_by_id_complete(contest_id);
    let grades = conn.get_contest_user_grades(session_token.clone(), contest_id);

    // TODO: Clean up star generation

    let mut tasks = Vec::new();
    for (task, grade) in c.taskgroups.into_iter().zip(grades) {
        let mut not_print_yet = true;
        let mut stasks = Vec::new();
        for st in task.tasks {
            let mut blackstars: usize = 0;
            if not_print_yet && st.stars >= grade.grade.unwrap_or(0) {
                blackstars = grade.grade.unwrap_or(0) as usize;
                not_print_yet = false;
            }

            stasks.push(SubTaskInfo { id: st.id.unwrap(),
                                      linktext: format!("{}{}",
                                                        str::repeat("★", blackstars as usize),
                                                        str::repeat("☆", st.stars as usize - blackstars as usize)) })
        }
        let mut ti = TaskInfo { name: task.name, subtasks: stasks };
        tasks.push(ti);
    }

    let ci = ContestInfo { id: c.id.unwrap(),
                           location: c.location.clone(),
                           filename: c.filename.clone(),
                           name: c.name.clone(),
                           duration: c.duration,
                           public: c.public,
                           tasks: tasks };

    let mut data = json_val::Map::new();
    data.insert("contest".to_string(), to_json(&ci));

    data.insert("logged_in".to_string(), to_json(&false));
    if let Some(session) = conn.get_session(&session_token) {
        data.insert("logged_in".to_string(), to_json(&true));
        data.insert("username".to_string(), to_json(&session.username));
        data.insert("firstname".to_string(), to_json(&session.firstname));
        data.insert("lastname".to_string(), to_json(&session.lastname));
        data.insert("teacher".to_string(), to_json(&session.is_teacher));
    }

    match conn.get_participation(&session_token, contest_id) {
        None => Ok(("contest".to_owned(), data)),
        Some(participation) => {
            let now = time::get_time();
            let passed_secs = now.sec - participation.start.sec;
            if passed_secs < 0 {
                // behandle inkonsistente Serverzeit
            }

            data.insert("participation_start_date".to_string(), to_json(&format!("{}", passed_secs)));

            let left_secs = i64::from(ci.duration) * 60 - passed_secs;
            if left_secs < 0 {
                // Contest over

            } else {
                let left_min = left_secs / 60;
                let left_sec = left_secs % 60;
                if left_sec < 10 {
                    data.insert("time_left".to_string(), to_json(&format!("{}:0{}", left_min, left_sec)));
                } else {
                    data.insert("time_left".to_string(), to_json(&format!("{}:{}", left_min, left_sec)));
                }
            }

            Ok(("contest".to_owned(), data))
        }
    }
}

pub fn show_contest_results<T: MedalConnection>(conn: &T, contest_id: u32, session_token: String) -> MedalValueResult {
    let session = conn.get_session(&session_token)
                      .ok_or(MedalError::AccessDenied)?
                      .ensure_alive()
                      .ok_or(MedalError::AccessDenied)?; // TODO SessionTimeout?
    let (tasknames, resultdata) = conn.get_contest_groups_grades(session.id, contest_id);

    let mut results: Vec<(String, Vec<(String, Vec<String>)>)> = Vec::new();

    for (group, groupdata) in resultdata {
        let mut groupresults: Vec<(String, Vec<String>)> = Vec::new();

        //TODO: use user
        for (_user, userdata) in groupdata {
            let mut userresults: Vec<String> = Vec::new();

            userresults.push(String::new());
            let mut summe = 0;

            for grade in userdata {
                if let Some(g) = grade.grade {
                    userresults.push(format!("{}", g));
                    summe += g;
                } else {
                    userresults.push(format!("–"));
                }
            }

            userresults[0] = format!("{}", summe);

            groupresults.push((format!("Name"), userresults))
        }

        results.push((format!("{}", group.name), groupresults));
    }

    let mut data = json_val::Map::new();
    data.insert("taskname".to_string(), to_json(&tasknames));
    data.insert("result".to_string(), to_json(&results));

    let c = conn.get_contest_by_id(contest_id);
    let ci = ContestInfo { id: c.id.unwrap(),
                           location: c.location.clone(),
                           filename: c.filename.clone(),
                           name: c.name.clone(),
                           duration: c.duration,
                           public: c.public,
                           tasks: Vec::new() };
    data.insert("contest".to_string(), to_json(&ci));

    Ok(("contestresults".to_owned(), data))
}

//TODO: use csrf_token
pub fn start_contest<T: MedalConnection>(conn: &T, contest_id: u32, session_token: String, _csrf_token: String)
                                         -> MedalResult<()> {
    //TODO: use data
    let _data = json_val::Map::new();

    match conn.new_participation(&session_token, contest_id) {
        Ok(_) => Ok(()),
        _ => Err(MedalError::AccessDenied),
    }
}

pub fn login<T: MedalConnection>(conn: &T, login_data: (String, String)) -> Result<String, MedalValue> {
    let (username, password) = login_data;

    match conn.login(None, &username, &password) {
        Ok(session_token) => Ok(session_token),
        Err(()) => {
            let mut data = json_val::Map::new();
            data.insert("reason".to_string(), to_json(&"Login fehlgeschlagen. Bitte erneut versuchen.".to_string()));
            data.insert("username".to_string(), to_json(&username));
            Err(("login".to_owned(), data))
        }
    }
}

pub fn login_with_code<T: MedalConnection>(
    conn: &T, code: String)
    -> Result<Result<String, String>, (String, json_val::Map<String, json_val::Value>)> {
    match conn.login_with_code(None, &code) {
        Ok(session_token) => Ok(Ok(session_token)),
        Err(()) => match conn.create_user_with_groupcode(None, &code) {
            Ok(session_token) => Ok(Err(session_token)),
            Err(()) => {
                let mut data = json_val::Map::new();
                data.insert("reason".to_string(), to_json(&"Kein gültiger Code. Bitte erneut versuchen.".to_string()));
                data.insert("code".to_string(), to_json(&code));
                Err(("login".to_owned(), data))
            }
        },
    }
}

pub fn logout<T: MedalConnection>(conn: &T, session_token: Option<String>) {
    session_token.map(|token| conn.logout(&token));
}

pub fn load_submission<T: MedalConnection>(conn: &T, task_id: u32, session_token: String, subtask: Option<String>)
                                           -> MedalResult<String> {
    let session = conn.get_session(&session_token)
                      .ok_or(MedalError::AccessDenied)?
                      .ensure_alive()
                      .ok_or(MedalError::AccessDenied)?; // TODO SessionTimeout

    match match subtask {
              Some(s) => conn.load_submission(&session, task_id, Some(&s)),
              None => conn.load_submission(&session, task_id, None),
          } {
        Some(submission) => Ok(submission.value),
        None => Ok("{}".to_string()),
    }
}

pub fn save_submission<T: MedalConnection>(conn: &T, task_id: u32, session_token: String, csrf_token: String,
                                           data: String, grade: u8, subtask: Option<String>)
                                           -> MedalResult<String>
{
    let session = conn.get_session(&session_token)
                      .ok_or(MedalError::AccessDenied)?
                      .ensure_alive()
                      .ok_or(MedalError::AccessDenied)?; // TODO SessionTimeout

    if session.csrf_token != csrf_token {
        return Err(MedalError::AccessDenied); // CsrfError
    }

    let submission = Submission { id: None,
                                  session_user: session.id,
                                  task: task_id,
                                  grade: grade,
                                  validated: false,
                                  nonvalidated_grade: grade,
                                  needs_validation: true,
                                  subtask_identifier: subtask,
                                  value: data,
                                  date: time::get_time() };

    conn.submit_submission(submission);

    Ok("{}".to_string())
}

pub fn show_task<T: MedalConnection>(conn: &T, task_id: u32, session_token: String) -> MedalValueResult {
    let session = conn.get_session_or_new(&session_token).ensure_alive().ok_or(MedalError::AccessDenied)?; // TODO SessionTimeout

    let (t, tg, c) = conn.get_task_by_id_complete(task_id);

    match conn.get_participation(&session_token, c.id.expect("Value from database")) {
        None => Err(MedalError::AccessDenied),
        Some(participation) => {
            let now = time::get_time();
            let passed_secs = now.sec - participation.start.sec;
            if passed_secs < 0 {
                // behandle inkonsistente Serverzeit
            }

            let mut data = json_val::Map::new();
            data.insert("participation_start_date".to_string(), to_json(&format!("{}", passed_secs)));

            let left_secs = i64::from(c.duration) * 60 - passed_secs;
            if left_secs < 0 {
                Err(MedalError::AccessDenied)
            // Contest over
            // TODO: Nicer message!
            } else {
                let (hour, min, sec) = (left_secs / 3600, left_secs / 60 % 60, left_secs % 60);

                data.insert("time_left".to_string(), to_json(&format!("{}:{:02}", hour, min)));
                data.insert("time_left_sec".to_string(), to_json(&format!(":{:02}", sec)));

                let taskpath = format!("{}{}", c.location, t.location);

                data.insert("name".to_string(), to_json(&tg.name));
                data.insert("taskid".to_string(), to_json(&task_id));
                data.insert("csrftoken".to_string(), to_json(&session.csrf_token));
                data.insert("taskpath".to_string(), to_json(&taskpath));
                data.insert("contestid".to_string(), to_json(&c.id));
                data.insert("seconds_left".to_string(), to_json(&left_secs));

                Ok(("task".to_owned(), data))
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct GroupInfo {
    pub id: u32,
    pub name: String,
    pub tag: String,
    pub code: String,
}

pub fn show_groups<T: MedalConnection>(conn: &T, session_token: String) -> MedalValueResult {
    let session = conn.get_session_or_new(&session_token).ensure_logged_in().ok_or(MedalError::NotLoggedIn)?;

    //    let groupvec = conn.get_group(session_token);

    let mut data = json_val::Map::new();

    let v: Vec<GroupInfo> =
        conn.get_groups(session.id)
            .iter()
            .map(|g| GroupInfo { id: g.id.unwrap(),
                                 name: g.name.clone(),
                                 tag: g.tag.clone(),
                                 code: g.groupcode.clone() })
            .collect();
    data.insert("group".to_string(), to_json(&v));
    data.insert("csrftoken".to_string(), to_json(&session.csrf_token));

    Ok(("groups".to_string(), data))
}

#[derive(Serialize, Deserialize)]
pub struct MemberInfo {
    pub id: u32,
    pub firstname: String,
    pub lastname: String,
    pub grade: u8,
    pub logincode: String,
}

pub fn show_group<T: MedalConnection>(conn: &T, group_id: u32, session_token: String) -> MedalValueResult {
    let session = conn.get_session_or_new(&session_token).ensure_logged_in().ok_or(MedalError::NotLoggedIn)?;
    let group = conn.get_group_complete(group_id).unwrap(); // TODO handle error

    let mut data = json_val::Map::new();

    if group.admin != session.id {
        return Err(MedalError::AccessDenied);
    }

    let gi = GroupInfo { id: group.id.unwrap(),
                         name: group.name.clone(),
                         tag: group.tag.clone(),
                         code: group.groupcode.clone() };

    let v: Vec<MemberInfo> =
        group.members
             .iter()
             .map(|m| MemberInfo { id: m.id,
                                   firstname: m.firstname.clone().unwrap_or_else(|| "".to_string()),
                                   lastname: m.lastname.clone().unwrap_or_else(|| "".to_string()),
                                   grade: m.grade,
                                   logincode: m.logincode.clone().unwrap_or_else(|| "".to_string()) })
             .collect();

    data.insert("group".to_string(), to_json(&gi));
    data.insert("member".to_string(), to_json(&v));

    Ok(("group".to_string(), data))
}

pub fn modify_group<T: MedalConnection>(_conn: &T, _group_id: u32, _session_token: String) -> MedalResult<()> {
    unimplemented!()
}

pub fn add_group<T: MedalConnection>(conn: &T, session_token: String, csrf_token: String, name: String, tag: String)
                                     -> MedalResult<u32> {
    let session = conn.get_session(&session_token)
                      .ok_or(MedalError::AccessDenied)?
                      .ensure_logged_in()
                      .ok_or(MedalError::AccessDenied)?;

    if session.csrf_token != csrf_token {
        return Err(MedalError::AccessDenied); // CsrfError
    }

    let group_code: String = Some('g').into_iter()
                                      .chain(thread_rng().sample_iter(&Alphanumeric))
                                      .filter(|x| {
                                          let x = *x;
                                          !(x == 'l' || x == 'I' || x == '1' || x == 'O' || x == 'o' || x == '0')
                                      })
                                      .take(7)
                                      .collect();
    // todo: check for collisions

    let mut group =
        Group { id: None, name: name, groupcode: group_code, tag: tag, admin: session.id, members: Vec::new() };

    conn.add_group(&mut group);

    Ok(group.id.unwrap())
}

#[allow(dead_code)]
pub fn show_groups_results<T: MedalConnection>(conn: &T, contest_id: u32, session_token: String) -> MedalValueResult {
    let session = conn.get_session_or_new(&session_token).ensure_logged_in().ok_or(MedalError::NotLoggedIn)?;
    //TODO: use g
    let _g = conn.get_contest_groups_grades(session.id, contest_id);

    let data = json_val::Map::new();

    Ok(("groupresults".into(), data))
}

pub fn show_profile<T: MedalConnection>(conn: &T, session_token: String, user_id: Option<u32>,
                                        query_string: Option<String>)
                                        -> MedalValueResult
{
    let session = conn.get_session_or_new(&session_token).ensure_alive().ok_or(MedalError::AccessDenied)?; // TODO SessionTimeout

    let mut data = json_val::Map::new();

    match user_id {
        None => {
            data.insert("firstname".to_string(), to_json(&session.firstname));
            data.insert("lastname".to_string(), to_json(&session.lastname));
            data.insert(format!("sel{}", session.grade), to_json(&"selected"));

            data.insert("logincode".to_string(), to_json(&session.logincode));
            if session.password.is_some() {
                data.insert("username".to_string(), to_json(&session.username));
            }
            data.insert("ownprofile".into(), to_json(&true));

            data.insert("csrftoken".to_string(), to_json(&session.csrf_token));

            if let Some(query) = query_string {
                if query.starts_with("status=") {
                    let status: &str = &query[7..];
                    if ["NothingChanged", "DataChanged", "PasswordChanged", "PasswordMissmatch"].contains(&status) {
                        data.insert((status).to_string(), to_json(&true));
                    }
                }
            }
        }
        Some(user_id) => {
            // TODO: Add test to check if this access restriction works
            let (user, opt_group) = conn.get_user_and_group_by_id(user_id).ok_or(MedalError::AccessDenied)?;
            let group = opt_group.ok_or(MedalError::AccessDenied)?;
            if group.admin != session.id {
                return Err(MedalError::AccessDenied);
            }

            data.insert("firstname".to_string(), to_json(&user.firstname));
            data.insert("lastname".to_string(), to_json(&user.lastname));
            data.insert(format!("sel{}", user.grade), to_json(&"selected"));

            data.insert("logincode".to_string(), to_json(&user.logincode));
            if user.password.is_some() {
                data.insert("username".to_string(), to_json(&user.username));
            }

            data.insert("ownprofile".into(), to_json(&false));

            data.insert("csrftoken".to_string(), to_json(&session.csrf_token));

            // data.insert("query_string".to_string(), to_json(&query_string.unwrap()));
        }
    }

    Ok(("profile".to_string(), data))
}

fn hash_password(password: &str, salt: &str) -> Result<String, MedalError> {
    let password_and_salt = [password, salt].concat().to_string();
    match hash(password_and_salt, 5) {
        Ok(result) => Ok(result),
        Err(_) => Err(MedalError::PasswordHashingError),
    }
}

#[derive(Debug)]
pub enum ProfileStatus {
    NothingChanged,
    DataChanged,
    PasswordChanged,
    PasswordMissmatch,
}
impl std::convert::Into<String> for ProfileStatus {
    fn into(self) -> String { format!("{:?}", self) }
}

pub fn edit_profile<T: MedalConnection>(conn: &T, session_token: String, user_id: Option<u32>, csrf_token: String,
                                        firstname: String, lastname: String, password: String,
                                        password_repeat: String, grade: u8)
                                        -> MedalResult<ProfileStatus>
{
    let mut session = conn.get_session(&session_token)
                          .ok_or(MedalError::AccessDenied)?
                          .ensure_alive()
                          .ok_or(MedalError::AccessDenied)?; // TODO SessionTimeout

    if session.csrf_token != csrf_token {
        return Err(MedalError::AccessDenied); // CsrfError
    }

    if session.firstname.as_ref() == Some(&firstname)
       && session.lastname.as_ref() == Some(&lastname)
       && session.grade == grade
       && password == ""
       && password_repeat == ""
    {
        return Ok(ProfileStatus::NothingChanged);
    }

    let mut result = ProfileStatus::DataChanged;

    let mut password_salt = None;

    if password != "" || password_repeat != "" {
        if password == password_repeat {
            let salt: String = thread_rng().sample_iter(&Alphanumeric).take(10).collect();
            let hash = hash_password(&password, &salt)?;

            password_salt = Some((hash, salt));
            result = ProfileStatus::PasswordChanged;
        } else {
            result = ProfileStatus::PasswordMissmatch;
        }
    }

    match user_id {
        None => {
            session.firstname = Some(firstname);
            session.lastname = Some(lastname);
            session.grade = grade;

            if let Some((password, salt)) = password_salt {
                session.password = Some(password);
                session.salt = Some(salt);
            }

            conn.save_session(session);
        }
        Some(user_id) => {
            // TODO: Add test to check if this access restriction works
            let (mut user, opt_group) = conn.get_user_and_group_by_id(user_id).ok_or(MedalError::AccessDenied)?;
            let group = opt_group.ok_or(MedalError::AccessDenied)?;
            if group.admin != session.id {
                return Err(MedalError::AccessDenied);
            }

            user.firstname = Some(firstname);
            user.lastname = Some(lastname);
            user.grade = grade;

            if let Some((password, salt)) = password_salt {
                user.password = Some(password);
                user.salt = Some(salt);
            }

            conn.save_session(user);
        }
    }

    Ok(result)
}

#[derive(PartialEq)]
pub enum UserType {
    User,
    Teacher,
    Admin,
}

pub enum UserGender {
    Female,
    Male,
    Unknown,
}

pub struct ForeignUserData {
    pub foreign_id: u32,
    pub foreign_type: UserType,
    pub gender: UserGender,
    pub firstname: String,
    pub lastname: String,
}

pub fn login_oauth<T: MedalConnection>(conn: &T, user_data: ForeignUserData)
                                       -> Result<String, (String, json_val::Map<String, json_val::Value>)> {
    match conn.login_foreign(None,
                             user_data.foreign_id,
                             user_data.foreign_type,
                             &user_data.firstname,
                             &user_data.lastname)
    {
        Ok(session_token) => Ok(session_token),
        Err(()) => {
            let mut data = json_val::Map::new();
            data.insert("reason".to_string(), to_json(&"OAuth-Login failed.".to_string()));
            Err(("login".to_owned(), data))
        }
    }
}

pub trait SetPassword {
    fn set_password(&mut self, &str) -> Option<()>;
}
impl SetPassword for SessionUser {
    fn set_password(&mut self, password: &str) -> Option<()> {
        let salt: String = thread_rng().sample_iter(&Alphanumeric).take(10).collect();
        let hash = hash_password(password, &salt).ok()?;

        self.password = Some(hash);
        self.salt = Some(salt);
        Some(())
    }
}
