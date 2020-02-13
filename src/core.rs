use time;

use db_conn::MedalConnection;
use db_objects::OptionSession;
use db_objects::SessionUser;
use db_objects::{Grade, Group, Participation, Submission, Taskgroup};
use helpers;
use oauth_provider::OauthProvider;
use webfw_iron::{json_val, to_json};

#[derive(Serialize, Deserialize)]
pub struct SubTaskInfo {
    pub id: i32,
    pub linktext: String,
    pub active: bool,
    pub greyout: bool,
}

#[derive(Serialize, Deserialize)]
pub struct TaskInfo {
    pub name: String,
    pub subtasks: Vec<SubTaskInfo>,
}

#[derive(Serialize, Deserialize)]
pub struct ContestInfo {
    pub id: i32,
    pub location: String,
    pub filename: String,
    pub name: String,
    pub duration: i32,
    pub public: bool,
    pub tasks: Vec<TaskInfo>,
}

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

fn fill_user_data(session: &SessionUser, data: &mut json_val::Map<String, serde_json::Value>) {
    if session.is_logged_in() {
        data.insert("logged_in".to_string(), to_json(&true));
        data.insert("username".to_string(), to_json(&session.username));
        data.insert("firstname".to_string(), to_json(&session.firstname));
        data.insert("lastname".to_string(), to_json(&session.lastname));
        data.insert("teacher".to_string(), to_json(&session.is_teacher));
        data.insert("csrf_token".to_string(), to_json(&session.csrf_token));
    }
    data.insert("parent".to_string(), to_json(&"base"));
}

pub fn index<T: MedalConnection>(conn: &T, session_token: Option<String>,
                                 (self_url, oauth_providers): (Option<String>, Option<Vec<OauthProvider>>))
                                 -> (String, json_val::Map<String, json_val::Value>)
{
    let mut data = json_val::Map::new();

    //let mut contests = Vec::new();

    if let Some(token) = session_token {
        if let Some(session) = conn.get_session(&token) {
            fill_user_data(&session, &mut data);
        }
    }

    let mut oauth_links: Vec<(String, String, String)> = Vec::new();
    if let Some(oauth_providers) = oauth_providers {
        for oauth_provider in oauth_providers {
            oauth_links.push((oauth_provider.provider_id.to_owned(),
                              oauth_provider.login_link_text.to_owned(),
                              oauth_provider.url.to_owned()));
        }
    }

    data.insert("self_url".to_string(), to_json(&self_url));
    data.insert("oauth_links".to_string(), to_json(&oauth_links));

    data.insert("parent".to_string(), to_json(&"base"));
    ("index".to_owned(), data)
}

pub fn debug<T: MedalConnection>(conn: &T, session_token: Option<String>)
                                 -> (String, json_val::Map<String, json_val::Value>) {
    let mut data = json_val::Map::new();

    if let Some(token) = session_token {
        if let Some(session) = conn.get_session(&token) {
            data.insert("known_session".to_string(), to_json(&true));
            data.insert("now_timestamp".to_string(), to_json(&time::get_time().sec));
            if let Some(last_activity) = session.last_activity {
                data.insert("session_timestamp".to_string(), to_json(&last_activity.sec));
                data.insert("timediff".to_string(), to_json(&(time::get_time() - last_activity).num_seconds()));
            }
            if session.is_alive() {
                data.insert("alive_session".to_string(), to_json(&true));
                if session.is_logged_in() {
                    data.insert("logged_in".to_string(), to_json(&true));
                    data.insert("username".to_string(), to_json(&session.username));
                    data.insert("firstname".to_string(), to_json(&session.firstname));
                    data.insert("lastname".to_string(), to_json(&session.lastname));
                    data.insert("teacher".to_string(), to_json(&session.is_teacher));
                }
            }
        }
        data.insert("session".to_string(), to_json(&token));
        println!("etwas session?!");
    } else {
        data.insert("session".to_string(), to_json(&"No session token given"));
        println!("warum nix session?!");
    }

    ("debug".to_owned(), data)
}

pub fn debug_create_session<T: MedalConnection>(conn: &T, session_token: Option<String>) {
    if let Some(token) = session_token {
        conn.get_session_or_new(&token);
    }
}

#[derive(PartialEq, Eq)]
pub enum ContestVisibility {
    All,
    Open,
    Current,
}

pub fn show_contests<T: MedalConnection>(conn: &T, session_token: &str,
                                         (self_url, oauth_providers): (Option<String>, Option<Vec<OauthProvider>>),
                                         visibility: ContestVisibility)
                                         -> MedalValue
{
    let mut data = json_val::Map::new();

    let session = conn.get_session_or_new(&session_token);
    fill_user_data(&session, &mut data);

    if session.is_logged_in() {
        data.insert("can_start".to_string(), to_json(&true));
    }

    let mut oauth_links: Vec<(String, String, String)> = Vec::new();
    if let Some(oauth_providers) = oauth_providers {
        for oauth_provider in oauth_providers {
            oauth_links.push((oauth_provider.provider_id.to_owned(),
                              oauth_provider.login_link_text.to_owned(),
                              oauth_provider.url.to_owned()));
        }
    }

    data.insert("self_url".to_string(), to_json(&self_url));
    data.insert("oauth_links".to_string(), to_json(&oauth_links));

    let v: Vec<ContestInfo> = conn.get_contest_list()
                                  .iter()
                                  .map(|c| ContestInfo { id: c.id.unwrap(),
                                                         location: c.location.clone(),
                                                         filename: c.filename.clone(),
                                                         name: c.name.clone(),
                                                         duration: c.duration,
                                                         public: c.public,
                                                         tasks: Vec::new() })
                                  .filter(|ci| ci.public)
                                  .filter(|ci| ci.duration == 0 || visibility != ContestVisibility::Open)
                                  .filter(|ci| ci.duration != 0 || visibility != ContestVisibility::Current)
                                  .collect();
    data.insert("contest".to_string(), to_json(&v));
    data.insert("contestlist_header".to_string(),
                to_json(&match visibility {
                            ContestVisibility::Open => "Trainingsaufgaben",
                            ContestVisibility::Current => "Aktuelle Wettbewerbe",
                            ContestVisibility::All => "Alle Wettbewerbe",
                        }));

    ("contests".to_owned(), data)
}

fn generate_subtaskstars(tg: &Taskgroup, grade: &Grade, ast: Option<i32>) -> Vec<SubTaskInfo> {
    let mut subtaskinfos = Vec::new();
    let mut not_print_yet = true;
    for st in &tg.tasks {
        let mut blackstars: usize = 0;
        if not_print_yet && st.stars >= grade.grade.unwrap_or(0) {
            blackstars = grade.grade.unwrap_or(0) as usize;
            not_print_yet = false;
        }

        let greyout = not_print_yet && st.stars < grade.grade.unwrap_or(0);
        let active = ast.is_some() && st.id == ast;
        let linktext = format!("{}{}",
                               str::repeat("★", blackstars as usize),
                               str::repeat("☆", st.stars as usize - blackstars as usize));
        let si = SubTaskInfo { id: st.id.unwrap(), linktext: linktext, active, greyout };

        subtaskinfos.push(si);
    }
    subtaskinfos
}

pub fn show_contest<T: MedalConnection>(conn: &T, contest_id: i32, session_token: &str, query_string: Option<String>)
                                        -> MedalValueResult {
    let session = conn.get_session_or_new(&session_token);

    let c = conn.get_contest_by_id_complete(contest_id);
    let grades = conn.get_contest_user_grades(&session_token, contest_id);

    let mut totalgrade = 0;
    let mut max_totalgrade = 0;

    let mut tasks = Vec::new();
    for (taskgroup, grade) in c.taskgroups.into_iter().zip(grades) {
        let subtaskstars = generate_subtaskstars(&taskgroup, &grade, None);
        let ti = TaskInfo { name: taskgroup.name, subtasks: subtaskstars };
        tasks.push(ti);

        totalgrade += grade.grade.unwrap_or(0);
        max_totalgrade += taskgroup.tasks.iter().map(|x| x.stars).max().unwrap_or(0);
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

    data.insert("logged_in".to_string(), to_json(&false)); // TODO: cant we just drop these two?
    data.insert("can_start".to_string(), to_json(&false));
    if session.is_logged_in() {
        data.insert("logged_in".to_string(), to_json(&true));
        data.insert("username".to_string(), to_json(&session.username));
        data.insert("firstname".to_string(), to_json(&session.firstname));
        data.insert("lastname".to_string(), to_json(&session.lastname));
        data.insert("teacher".to_string(), to_json(&session.is_teacher));
        data.insert("csrf_token".to_string(), to_json(&session.csrf_token));
    }
    if c.duration == 0 || session.is_logged_in() {
        data.insert("can_start".to_string(), to_json(&true));

        if let Some(start_date) = c.start {
            if time::get_time() < start_date {
                data.insert("can_start".to_string(), to_json(&false));
            }
        }
        if let Some(end_date) = c.end {
            if time::get_time() > end_date {
                data.insert("can_start".to_string(), to_json(&false));
            }
        }

        let student_grade = session.grade % 100 - if session.grade / 100 == 1 { 1 } else { 0 };

        if c.min_grade.map(|ming| student_grade < ming).unwrap_or(false) {
            data.insert("can_start".to_string(), to_json(&false));
            data.insert("grade_too_low".to_string(), to_json(&true));
        }

        if c.max_grade.map(|maxg| student_grade > maxg).unwrap_or(false) {
            data.insert("can_start".to_string(), to_json(&false));
            data.insert("grade_too_high".to_string(), to_json(&true));
        }
    }

    if let Some(start_date) = c.start {
        if time::get_time() < start_date {
            data.insert("can_start".to_string(), to_json(&false));

            let time_until = start_date - time::get_time();
            data.insert("time_until_d".to_string(), to_json(&(time_until.num_days())));
            data.insert("time_until_h".to_string(), to_json(&(time_until.num_hours() % 24)));
            data.insert("time_until_m".to_string(), to_json(&(time_until.num_minutes() % 60)));
        }
    }

    // This only checks if a query string is existent, so any query string will
    // lead to the assumption that a bare page is requested. This is useful to
    // disable caching (via random token) but should be changed if query string
    // can obtain more than only this meaning in the future
    if query_string.is_none() {
        data.insert("not_bare".to_string(), to_json(&true));
    }

    let mut opt_part = conn.get_participation(&session_token, contest_id);

    // Autostart if appropriate
    // TODO: Should participation start automatically for teacher? Even before the contest start?
    // Should teachers have all time access or only the same limited amount of time?
    // if opt_part.is_none() && (c.duration == 0 || session.is_teacher) {
    // TODO: Should autostart only happen in the contest time?
    if opt_part.is_none() && c.duration == 0 {
        conn.new_participation(&session_token, contest_id).map_err(|_| MedalError::AccessDenied)?;
        opt_part = Some(Participation { contest: contest_id, user: session.id, start: time::get_time() });
    }

    if let Some(participation) = opt_part {
        let now = time::get_time();
        let passed_secs = now.sec - participation.start.sec;
        if passed_secs < 0 {
            // behandle inkonsistente Serverzeit
        }

        data.insert("started".to_string(), to_json(&true));
        data.insert("participation_start_date".to_string(), to_json(&format!("{}", passed_secs)));
        data.insert("total_points".to_string(), to_json(&totalgrade));
        data.insert("max_total_points".to_string(), to_json(&max_totalgrade));
        data.insert("relative_points".to_string(), to_json(&((totalgrade * 100) / max_totalgrade)));

        let left_secs = i64::from(ci.duration) * 60 - passed_secs;
        if left_secs < 0 {
            // Contest over
            data.insert("is_time_left".to_string(), to_json(&false));
            if c.duration > 0 {
                data.insert("is_time_up".to_string(), to_json(&true));
            }
        } else {
            data.insert("is_time_left".to_string(), to_json(&true));
            let left_min = left_secs / 60;
            let left_sec = left_secs % 60;
            if left_sec < 10 {
                data.insert("time_left".to_string(), to_json(&format!("{}:0{}", left_min, left_sec)));
            } else {
                data.insert("time_left".to_string(), to_json(&format!("{}:{}", left_min, left_sec)));
            }
        }
    }

    if c.duration > 0 {
        data.insert("duration".to_string(), to_json(&true));
    }

    Ok(("contest".to_owned(), data))
}

pub fn show_contest_results<T: MedalConnection>(conn: &T, contest_id: i32, session_token: &str) -> MedalValueResult {
    let session = conn.get_session(&session_token).ensure_logged_in().ok_or(MedalError::NotLoggedIn)?;
    let mut data = json_val::Map::new();
    fill_user_data(&session, &mut data);

    let (tasknames, resultdata) = conn.get_contest_groups_grades(session.id, contest_id);

    let mut results: Vec<(String, i32, Vec<(String, i32, Vec<String>)>)> = Vec::new();

    for (group, groupdata) in resultdata {
        let mut groupresults: Vec<(String, i32, Vec<String>)> = Vec::new();

        //TODO: use user
        for (user, userdata) in groupdata {
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

            groupresults.push((format!("{} {}",
                                       user.firstname.unwrap_or_default(),
                                       user.lastname.unwrap_or_else(|| "–".to_string())),
                               user.id,
                               userresults))
        }

        results.push((format!("{}", group.name), group.id.unwrap_or(0), groupresults));
    }

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

pub fn start_contest<T: MedalConnection>(conn: &T, contest_id: i32, session_token: &str, csrf_token: &str)
                                         -> MedalResult<()> {
    // TODO: Is _or_new the right semantic? We need a CSRF token anyway …
    let session = conn.get_session_or_new(&session_token);
    let c = conn.get_contest_by_id(contest_id);

    // Check contest currently available:
    if let Some(start_date) = c.start {
        if time::get_time() < start_date {
            return Err(MedalError::AccessDenied);
        }
    }
    if let Some(end_date) = c.end {
        if time::get_time() > end_date {
            return Err(MedalError::AccessDenied);
        }
    }

    // TODO: Check participant is in correct age group (not super important)

    // Check logged in or open contest
    if c.duration != 0 && !session.is_logged_in() {
        return Err(MedalError::AccessDenied);
    }

    // Check CSRF token
    if session.is_logged_in() && session.csrf_token != csrf_token {
        return Err(MedalError::CsrfCheckFailed);
    }

    // Start contest
    match conn.new_participation(&session_token, contest_id) {
        Ok(_) => Ok(()),
        _ => Err(MedalError::AccessDenied), // Contest already started TODO: Maybe redirect to page with hint
    }
}

pub fn login<T: MedalConnection>(conn: &T, login_data: (String, String),
                                 (self_url, oauth_providers): (Option<String>, Option<Vec<OauthProvider>>))
                                 -> Result<String, MedalValue>
{
    let (username, password) = login_data;

    match conn.login(None, &username, &password) {
        Ok(session_token) => Ok(session_token),
        Err(()) => {
            let mut data = json_val::Map::new();
            data.insert("reason".to_string(), to_json(&"Login fehlgeschlagen. Bitte erneut versuchen.".to_string()));
            data.insert("username".to_string(), to_json(&username));
            data.insert("parent".to_string(), to_json(&"base"));

            let mut oauth_links: Vec<(String, String, String)> = Vec::new();
            if let Some(oauth_providers) = oauth_providers {
                for oauth_provider in oauth_providers {
                    oauth_links.push((oauth_provider.provider_id.to_owned(),
                                      oauth_provider.login_link_text.to_owned(),
                                      oauth_provider.url.to_owned()));
                }
            }

            data.insert("self_url".to_string(), to_json(&self_url));
            data.insert("oauth_links".to_string(), to_json(&oauth_links));

            Err(("login".to_owned(), data))
        }
    }
}

pub fn login_with_code<T: MedalConnection>(
    conn: &T, code: &str, (self_url, oauth_providers): (Option<String>, Option<Vec<OauthProvider>>))
    -> Result<Result<String, String>, (String, json_val::Map<String, json_val::Value>)> {
    match conn.login_with_code(None, &code) {
        Ok(session_token) => Ok(Ok(session_token)),
        Err(()) => match conn.create_user_with_groupcode(None, &code) {
            Ok(session_token) => Ok(Err(session_token)),
            Err(()) => {
                let mut data = json_val::Map::new();
                data.insert("reason".to_string(), to_json(&"Kein gültiger Code. Bitte erneut versuchen.".to_string()));
                data.insert("code".to_string(), to_json(&code));
                data.insert("parent".to_string(), to_json(&"base"));

                let mut oauth_links: Vec<(String, String, String)> = Vec::new();
                if let Some(oauth_providers) = oauth_providers {
                    for oauth_provider in oauth_providers {
                        oauth_links.push((oauth_provider.provider_id.to_owned(),
                                          oauth_provider.login_link_text.to_owned(),
                                          oauth_provider.url.to_owned()));
                    }
                }

                data.insert("self_url".to_string(), to_json(&self_url));
                data.insert("oauth_links".to_string(), to_json(&oauth_links));

                Err(("login".to_owned(), data))
            }
        },
    }
}

pub fn logout<T: MedalConnection>(conn: &T, session_token: Option<String>) {
    session_token.map(|token| conn.logout(&token));
}

pub fn load_submission<T: MedalConnection>(conn: &T, task_id: i32, session_token: &str, subtask: Option<String>)
                                           -> MedalResult<String> {
    let session = conn.get_session(&session_token).ensure_alive().ok_or(MedalError::NotLoggedIn)?;

    match match subtask {
              Some(s) => conn.load_submission(&session, task_id, Some(&s)),
              None => conn.load_submission(&session, task_id, None),
          } {
        Some(submission) => Ok(submission.value),
        None => Ok("{}".to_string()),
    }
}

pub fn save_submission<T: MedalConnection>(conn: &T, task_id: i32, session_token: &str, csrf_token: &str,
                                           data: String, grade: i32, subtask: Option<String>)
                                           -> MedalResult<String>
{
    let session = conn.get_session(&session_token).ensure_alive().ok_or(MedalError::NotLoggedIn)?;

    if session.csrf_token != csrf_token {
        return Err(MedalError::CsrfCheckFailed);
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

pub fn show_task<T: MedalConnection>(conn: &T, task_id: i32, session_token: &str) -> MedalValueResult {
    let session = conn.get_session_or_new(&session_token);

    let (t, tg, c) = conn.get_task_by_id_complete(task_id);
    let grade = conn.get_taskgroup_user_grade(&session_token, tg.id.unwrap()); // TODO: Unwrap?
    let tasklist = conn.get_contest_by_id_complete(c.id.unwrap()); // TODO: Unwrap?

    let mut prevtaskgroup: Option<Taskgroup> = None;
    let mut nexttaskgroup: Option<Taskgroup> = None;
    let mut current_found = false;

    let mut subtaskstars = Vec::new();

    for taskgroup in tasklist.taskgroups {
        if current_found {
            nexttaskgroup = Some(taskgroup);
            break;
        }

        if taskgroup.id == tg.id {
            current_found = true;
            subtaskstars = generate_subtaskstars(&taskgroup, &grade, Some(task_id));
        } else {
            prevtaskgroup = Some(taskgroup);
        }
    }

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
            data.insert("subtasks".to_string(), to_json(&subtaskstars));
            data.insert("prevtask".to_string(), to_json(&prevtaskgroup.map(|tg| tg.tasks[0].id)));
            data.insert("nexttask".to_string(), to_json(&nexttaskgroup.map(|tg| tg.tasks[0].id))); // TODO: fail better

            let left_secs = i64::from(c.duration) * 60 - passed_secs;
            if c.duration > 0 && left_secs < 0 {
                Err(MedalError::AccessDenied)
            // Contest over
            // TODO: Nicer message!
            } else {
                let (hour, min, sec) = (left_secs / 3600, left_secs / 60 % 60, left_secs % 60);

                data.insert("time_left".to_string(), to_json(&format!("{}:{:02}", hour, min)));
                data.insert("time_left_sec".to_string(), to_json(&format!(":{:02}", sec)));

                let taskpath = format!("{}{}", c.location, t.location);

                data.insert("contestname".to_string(), to_json(&c.name));
                data.insert("name".to_string(), to_json(&tg.name));
                data.insert("taskid".to_string(), to_json(&task_id));
                data.insert("csrf_token".to_string(), to_json(&session.csrf_token));
                data.insert("taskpath".to_string(), to_json(&taskpath));
                data.insert("contestid".to_string(), to_json(&c.id));
                data.insert("seconds_left".to_string(), to_json(&left_secs));

                if c.duration > 0 {
                    data.insert("duration".to_string(), to_json(&true));
                }

                Ok(("task".to_owned(), data))
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct GroupInfo {
    pub id: i32,
    pub name: String,
    pub tag: String,
    pub code: String,
}

pub fn show_groups<T: MedalConnection>(conn: &T, session_token: &str) -> MedalValueResult {
    let session = conn.get_session(&session_token).ensure_logged_in().ok_or(MedalError::NotLoggedIn)?;

    //    let groupvec = conn.get_group(session_token);

    let mut data = json_val::Map::new();
    fill_user_data(&session, &mut data);

    let v: Vec<GroupInfo> =
        conn.get_groups(session.id)
            .iter()
            .map(|g| GroupInfo { id: g.id.unwrap(),
                                 name: g.name.clone(),
                                 tag: g.tag.clone(),
                                 code: g.groupcode.clone() })
            .collect();
    data.insert("group".to_string(), to_json(&v));
    data.insert("csrf_token".to_string(), to_json(&session.csrf_token));

    Ok(("groups".to_string(), data))
}

#[derive(Serialize, Deserialize)]
pub struct MemberInfo {
    pub id: i32,
    pub firstname: String,
    pub lastname: String,
    pub grade: String,
    pub logincode: String,
}

pub fn show_group<T: MedalConnection>(conn: &T, group_id: i32, session_token: &str) -> MedalValueResult {
    let session = conn.get_session(&session_token).ensure_logged_in().ok_or(MedalError::NotLoggedIn)?;
    let group = conn.get_group_complete(group_id).unwrap(); // TODO handle error

    let mut data = json_val::Map::new();
    fill_user_data(&session, &mut data);

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
                                   grade: match m.grade {
                                       0 => "Noch kein Schüler".to_string(),
                                       n @ 1..=10 => format!("{}", n),
                                       11 => "11 (G8)".to_string(),
                                       12 => "12 (G8)".to_string(),
                                       111 => "11 (G9)".to_string(),
                                       112 => "12 (G9)".to_string(),
                                       113 => "13 (G9)".to_string(),
                                       114 => "Berufsschule".to_string(),
                                       255 => "Kein Schüler mehr".to_string(),
                                       _ => "?".to_string(),
                                   },
                                   logincode: m.logincode.clone().unwrap_or_else(|| "".to_string()) })
             .collect();

    data.insert("group".to_string(), to_json(&gi));
    data.insert("member".to_string(), to_json(&v));
    data.insert("groupname".to_string(), to_json(&gi.name));

    Ok(("group".to_string(), data))
}

pub fn modify_group<T: MedalConnection>(_conn: &T, _group_id: i32, _session_token: &str) -> MedalResult<()> {
    unimplemented!()
}

pub fn add_group<T: MedalConnection>(conn: &T, session_token: &str, csrf_token: &str, name: String, tag: String)
                                     -> MedalResult<i32> {
    let session = conn.get_session(&session_token).ensure_logged_in().ok_or(MedalError::AccessDenied)?;

    if session.csrf_token != csrf_token {
        return Err(MedalError::CsrfCheckFailed);
    }

    let group_code = helpers::make_group_code();
    // TODO: check for collisions

    let mut group =
        Group { id: None, name: name, groupcode: group_code, tag: tag, admin: session.id, members: Vec::new() };

    conn.add_group(&mut group);

    Ok(group.id.unwrap())
}

pub fn group_csv<T: MedalConnection>(conn: &T, session_token: &str) -> MedalValueResult {
    let session = conn.get_session(&session_token).ensure_logged_in().ok_or(MedalError::NotLoggedIn)?;

    let mut data = json_val::Map::new();
    data.insert("csrf_token".to_string(), to_json(&session.csrf_token));

    Ok(("groupcsv".to_string(), data))
}

// TODO: Should creating the users and groups happen in a batch operation to speed things up?
pub fn upload_groups<T: MedalConnection>(conn: &T, session_token: &str, csrf_token: &str, group_data: &str)
                                         -> MedalResult<()> {
    let session = conn.get_session(&session_token).ensure_logged_in().ok_or(MedalError::NotLoggedIn)?;

    if session.csrf_token != csrf_token {
        return Err(MedalError::CsrfCheckFailed);
    }

    println!("{}", group_data);

    let mut v: Vec<Vec<String>> = serde_json::from_str(group_data).or(Err(MedalError::AccessDenied))?; // TODO: Change error type
    v.sort_unstable_by(|a, b| a[0].partial_cmp(&b[0]).unwrap());

    let mut group_code = "".to_string();
    let mut name = "".to_string();
    let mut group = Group { id: None,
                            name: name.clone(),
                            groupcode: group_code,
                            tag: "".to_string(),
                            admin: session.id,
                            members: Vec::new() };

    for line in v {
        if name != line[0] {
            if name != "" {
                conn.create_group_with_users(group);
            }
            name = line[0].clone();
            group_code = helpers::make_group_code();
            // TODO: check for collisions

            group = Group { id: None,
                            name: name.clone(),
                            groupcode: group_code,
                            tag: name.clone(),
                            admin: session.id,
                            members: Vec::new() };
        }

        let mut user = SessionUser::group_user_stub();
        user.grade = line[1].parse::<i32>().unwrap_or(0);
        user.firstname = Some(line[2].clone());
        user.lastname = Some(line[3].clone());

        group.members.push(user);
    }
    conn.create_group_with_users(group);

    Ok(())
}

#[allow(dead_code)]
pub fn show_groups_results<T: MedalConnection>(conn: &T, contest_id: i32, session_token: &str) -> MedalValueResult {
    let session = conn.get_session(&session_token).ensure_logged_in().ok_or(MedalError::NotLoggedIn)?;
    //TODO: use g
    let _g = conn.get_contest_groups_grades(session.id, contest_id);

    let data = json_val::Map::new();

    Ok(("groupresults".into(), data))
}

pub fn show_profile<T: MedalConnection>(conn: &T, session_token: &str, user_id: Option<i32>,
                                        query_string: Option<String>)
                                        -> MedalValueResult
{
    let session = conn.get_session(&session_token).ensure_logged_in().ok_or(MedalError::NotLoggedIn)?;

    let mut data = json_val::Map::new();
    fill_user_data(&session, &mut data);

    match user_id {
        None => {
            data.insert("profile_firstname".to_string(), to_json(&session.firstname));
            data.insert("profile_lastname".to_string(), to_json(&session.lastname));
            data.insert("profile_street".to_string(), to_json(&session.street));
            data.insert("profile_zip".to_string(), to_json(&session.zip));
            data.insert("profile_city".to_string(), to_json(&session.city));
            data.insert(format!("sel{}", session.grade), to_json(&"selected"));

            data.insert("profile_logincode".to_string(), to_json(&session.logincode));
            if session.password.is_some() {
                data.insert("profile_username".to_string(), to_json(&session.username));
                data.insert("profile_not_in_group".into(), to_json(&true));
            }
            data.insert("ownprofile".into(), to_json(&true));

            if let Some(query) = query_string {
                if query.starts_with("status=") {
                    let status: &str = &query[7..];
                    if ["NothingChanged", "DataChanged", "PasswordChanged", "PasswordMissmatch", "firstlogin"].contains(&status) {
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

            data.insert("profile_firstname".to_string(), to_json(&user.firstname));
            data.insert("profile_lastname".to_string(), to_json(&user.lastname));
            data.insert("profile_street".to_string(), to_json(&session.street));
            data.insert("profile_zip".to_string(), to_json(&session.zip));
            data.insert("profile_city".to_string(), to_json(&session.city));
            data.insert(format!("sel{}", user.grade), to_json(&"selected"));

            data.insert("profile_logincode".to_string(), to_json(&user.logincode));
            if user.password.is_some() {
                data.insert("profile_username".to_string(), to_json(&user.username));
                data.insert("profile_not_in_group".into(), to_json(&true));
            }

            data.insert("ownprofile".into(), to_json(&false));

            if let Some(query) = query_string {
                if query.starts_with("status=") {
                    let status: &str = &query[7..];
                    if ["NothingChanged", "DataChanged", "PasswordChanged", "PasswordMissmatch"].contains(&status) {
                        data.insert((status).to_string(), to_json(&true));
                    }
                }
            }
        }
    }

    Ok(("profile".to_string(), data))
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

pub fn edit_profile<T: MedalConnection>(conn: &T, session_token: &str, user_id: Option<i32>, csrf_token: &str,
                                        (firstname, lastname, street, zip, city, password, password_repeat, grade): (
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        i32,
    ))
                                        -> MedalResult<ProfileStatus>
{
    let mut session = conn.get_session(&session_token).ensure_logged_in().ok_or(MedalError::NotLoggedIn)?;

    if session.csrf_token != csrf_token {
        return Err(MedalError::AccessDenied); // CsrfError
    }

    if session.firstname.as_ref() == Some(&firstname)
       && session.lastname.as_ref() == Some(&lastname)
       && session.street == street
       && session.zip == zip
       && session.city == city
       && session.grade == grade
    {
        return Ok(ProfileStatus::NothingChanged);
    }

    let mut result = ProfileStatus::DataChanged;

    let mut password_and_salt = None;

    if let (Some(password), Some(password_repeat)) = (password, password_repeat) {
        if password != "" || password_repeat != "" {
            if password == password_repeat {
                let salt = helpers::make_salt();
                let hash = helpers::hash_password(&password, &salt)?;

                password_and_salt = Some((hash, salt));
                result = ProfileStatus::PasswordChanged;
            } else {
                result = ProfileStatus::PasswordMissmatch;
            }
        } else {
            return Ok(ProfileStatus::NothingChanged);
        }
    }

    match user_id {
        None => {
            session.firstname = Some(firstname);
            session.lastname = Some(lastname);
            session.grade = grade;

            if street.is_some() {
                session.street = street;
            }
            if zip.is_some() {
                session.zip = zip;
            }
            if city.is_some() {
                session.city = city;
            }

            if let Some((password, salt)) = password_and_salt {
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

            if street.is_some() {
                user.street = street;
            }
            if zip.is_some() {
                user.zip = zip;
            }
            if city.is_some() {
                user.city = city;
            }

            if let Some((password, salt)) = password_and_salt {
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
    pub foreign_id: String,
    pub foreign_type: UserType,
    pub gender: UserGender,
    pub firstname: String,
    pub lastname: String,
}

pub fn login_oauth<T: MedalConnection>(conn: &T, user_data: ForeignUserData)
                                       -> Result<String, (String, json_val::Map<String, json_val::Value>)> {
    match conn.login_foreign(None,
                             &user_data.foreign_id,
                             user_data.foreign_type != UserType::User,
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
