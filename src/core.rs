/*  medal                                                                                                            *\
 *  Copyright (C) 2022  Bundesweite Informatikwettbewerbe, Robert Czechowski                                                            *
 *                                                                                                                   *
 *  This program is free software: you can redistribute it and/or modify it under the terms of the GNU Affero        *
 *  General Public License as published  by the Free Software Foundation, either version 3 of the License, or (at    *
 *  your option) any later version.                                                                                  *
 *                                                                                                                   *
 *  This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the       *
 *  implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public      *
 *  License for more details.                                                                                        *
 *                                                                                                                   *
 *  You should have received a copy of the GNU Affero General Public License along with this program.  If not, see   *
\*  <http://www.gnu.org/licenses/>.                                                                                  */

use time;

use config::OauthProvider;
use db_conn::MedalConnection;
#[cfg(feature = "signup")]
use db_conn::SignupResult;
use db_objects::OptionSession;
use db_objects::SessionUser;
use db_objects::{Contest, Grade, Group, Participation, Submission, Taskgroup};
use helpers;
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

#[derive(Clone, Serialize, Deserialize)]
pub struct ContestInfo {
    pub id: i32,
    pub name: String,
    pub duration: i32,
    pub public: bool,
    pub requires_login: bool,
    pub image: Option<String>,
    pub language: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Clone, Debug)]
pub enum MedalError {
    NotLoggedIn,
    AccessDenied,
    CsrfCheckFailed,
    SessionTimeout,
    DatabaseError,
    ConfigurationError,
    DatabaseConnectionError,
    PasswordHashingError,
    UnmatchedPasswords,
    NotFound,
    AccountIncomplete,
    UnknownId,
    OauthError(String),
}

pub struct LoginInfo {
    pub password_login: bool,
    pub self_url: Option<String>,
    pub oauth_providers: Option<Vec<OauthProvider>>,
}

type MedalValue = (String, json_val::Map<String, json_val::Value>);
type MedalResult<T> = Result<T, MedalError>;
type MedalValueResult = MedalResult<MedalValue>;

fn fill_user_data_prefix(session: &SessionUser, data: &mut json_val::Map<String, serde_json::Value>, prefix: &str) {
    data.insert(prefix.to_string() + "username", to_json(&session.username));
    data.insert(prefix.to_string() + "firstname", to_json(&session.firstname));
    data.insert(prefix.to_string() + "lastname", to_json(&session.lastname));
    data.insert(prefix.to_string() + "teacher", to_json(&session.is_teacher));
    data.insert(prefix.to_string() + "is_teacher", to_json(&session.is_teacher));
    data.insert(prefix.to_string() + "admin", to_json(&session.is_admin));
    data.insert(prefix.to_string() + "is_admin", to_json(&session.is_admin));
    data.insert(prefix.to_string() + "logged_in", to_json(&session.is_logged_in()));
    data.insert(prefix.to_string() + "csrf_token", to_json(&session.csrf_token));
    data.insert(prefix.to_string() + "sex",
                to_json(&(match session.sex {
                            Some(0) | None => "/",
                            Some(1) => "m",
                            Some(2) => "w",
                            Some(3) => "d",
                            Some(4) => "…",
                            _ => "?",
                        })));
}

fn fill_user_data(session: &SessionUser, data: &mut json_val::Map<String, serde_json::Value>) {
    fill_user_data_prefix(session, data, "");

    data.insert("parent".to_string(), to_json(&"base"));
    data.insert("medal_version".to_string(), to_json(&env!("CARGO_PKG_VERSION")));
}

fn fill_oauth_data(login_info: LoginInfo, data: &mut json_val::Map<String, serde_json::Value>) {
    let mut oauth_links: Vec<(String, String, String)> = Vec::new();
    if let Some(oauth_providers) = login_info.oauth_providers {
        for oauth_provider in oauth_providers {
            oauth_links.push((oauth_provider.provider_id.to_owned(),
                              oauth_provider.login_link_text.to_owned(),
                              oauth_provider.url.to_owned()));
        }
    }

    data.insert("self_url".to_string(), to_json(&login_info.self_url));
    data.insert("oauth_links".to_string(), to_json(&oauth_links));

    data.insert("password_login".to_string(), to_json(&login_info.password_login));
}

fn grade_to_string(grade: i32) -> String {
    match grade {
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
    }
}

pub fn index<T: MedalConnection>(conn: &T, session_token: Option<String>, login_info: LoginInfo) -> MedalValueResult {
    let mut data = json_val::Map::new();

    if let Some(token) = session_token {
        if let Some(session) = conn.get_session(&token) {
            fill_user_data(&session, &mut data);

            if session.logincode.is_some() && session.firstname.is_none() {
                return Err(MedalError::AccountIncomplete);
            }
        }
    }

    fill_oauth_data(login_info, &mut data);

    data.insert("parent".to_string(), to_json(&"base"));
    data.insert("index".to_string(), to_json(&true));
    Ok(("index".to_owned(), data))
}

pub fn show_login<T: MedalConnection>(conn: &T, session_token: Option<String>, login_info: LoginInfo)
                                      -> (String, json_val::Map<String, json_val::Value>) {
    let mut data = json_val::Map::new();

    if let Some(token) = session_token {
        if let Some(session) = conn.get_session(&token) {
            fill_user_data(&session, &mut data);
        }
    }

    fill_oauth_data(login_info, &mut data);

    data.insert("parent".to_string(), to_json(&"base"));
    ("login".to_owned(), data)
}

pub fn status<T: MedalConnection>(conn: &T, config_secret: Option<String>, given_secret: Option<String>)
                                  -> MedalResult<String> {
    if config_secret == given_secret {
        Ok(conn.get_debug_information())
    } else {
        Err(MedalError::AccessDenied)
    }
}

pub fn debug<T: MedalConnection>(conn: &T, session_token: Option<String>)
                                 -> (String, json_val::Map<String, json_val::Value>) {
    let mut data = json_val::Map::new();

    if let Some(token) = session_token {
        if let Some(session) = conn.get_session(&token) {
            data.insert("known_session".to_string(), to_json(&true));
            data.insert("session_id".to_string(), to_json(&session.id));
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
                    data.insert("oauth_provider".to_string(), to_json(&session.oauth_provider));
                    data.insert("oauth_id".to_string(), to_json(&session.oauth_foreign_id));
                    data.insert("logincode".to_string(), to_json(&session.logincode));
                    data.insert("managed_by".to_string(), to_json(&session.managed_by));
                }
            }
        }
        data.insert("session".to_string(), to_json(&token));
    } else {
        data.insert("session".to_string(), to_json(&"No session token given"));
    }

    ("debug".to_owned(), data)
}

pub fn debug_create_session<T: MedalConnection>(conn: &T, session_token: Option<String>) {
    if let Some(token) = session_token {
        conn.get_session_or_new(&token).unwrap();
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum ContestVisibility {
    All,
    Open,
    Current,
    LoginRequired,
    StandaloneTask,
}

pub fn show_contests<T: MedalConnection>(conn: &T, session_token: &str, login_info: LoginInfo,
                                         visibility: ContestVisibility)
                                         -> MedalValueResult {
    let mut data = json_val::Map::new();

    let session = conn.get_session_or_new(&session_token).map_err(|_| MedalError::DatabaseConnectionError)?;
    fill_user_data(&session, &mut data);

    if session.is_logged_in() {
        data.insert("can_start".to_string(), to_json(&true));
    }

    fill_oauth_data(login_info, &mut data);

    let now = time::get_time();
    let v: Vec<ContestInfo> =
        conn.get_contest_list()
            .iter()
            .filter(|c| c.public)
            .filter(|c| (!c.standalone_task.unwrap_or(false)) || visibility == ContestVisibility::StandaloneTask)
            .filter(|c| c.standalone_task.unwrap_or(false) || visibility != ContestVisibility::StandaloneTask)
            .filter(|c| c.end.map(|end| now <= end).unwrap_or(true) || visibility == ContestVisibility::All)
            .filter(|c| c.duration == 0 || visibility != ContestVisibility::Open)
            .filter(|c| c.duration != 0 || visibility != ContestVisibility::Current)
            .filter(|c| c.requires_login.unwrap_or(false) || visibility != ContestVisibility::LoginRequired)
            .filter(|c| {
                !c.requires_login.unwrap_or(false)
                || visibility == ContestVisibility::LoginRequired
                || visibility == ContestVisibility::All
            })
            .map(|c| ContestInfo { id: c.id.unwrap(),
                                   name: c.name.clone(),
                                   duration: c.duration,
                                   public: c.public,
                                   requires_login: c.requires_login.unwrap_or(false),
                                   image: c.image.as_ref().map(|i| format!("/{}{}", c.location, i)),
                                   language: c.language.clone(),
                                   tags: c.tags.clone() })
            .collect();

    let contests_training: Vec<ContestInfo> =
        v.clone().into_iter().filter(|c| !c.requires_login).filter(|c| c.duration == 0).collect();
    let contests_contest: Vec<ContestInfo> =
        v.clone().into_iter().filter(|c| !c.requires_login).filter(|c| c.duration != 0).collect();
    let contests_challenge: Vec<ContestInfo> = v.into_iter().filter(|c| c.requires_login).collect();

    data.insert("contests_training".to_string(), to_json(&contests_training));
    data.insert("contests_contest".to_string(), to_json(&contests_contest));
    data.insert("contests_challenge".to_string(), to_json(&contests_challenge));

    data.insert("contests_training_header".to_string(), to_json(&"Trainingsaufgaben"));
    data.insert("contests_contest_header".to_string(), to_json(&"Wettbewerbe"));
    data.insert("contests_challenge_header".to_string(), to_json(&"Herausforderungen"));

    if visibility == ContestVisibility::StandaloneTask {
        data.insert("contests_training_header".to_string(), to_json(&"Einzelne Aufgaben ohne Wertung"));
    }

    Ok(("contests".to_owned(), data))
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
        let si = SubTaskInfo { id: st.id.unwrap(), linktext, active, greyout };

        subtaskinfos.push(si);
    }
    subtaskinfos
}

#[derive(Serialize, Deserialize)]
pub struct ContestStartConstraints {
    pub contest_not_begun: bool,
    pub contest_over: bool,
    pub contest_running: bool,
    pub grade_too_low: bool,
    pub grade_too_high: bool,
    pub grade_matching: bool,
}

fn check_contest_qualification<T: MedalConnection>(conn: &T, session: &SessionUser, contest: &Contest) -> Option<bool> {
    // Produced by `config.requires_contest.map(|list| list.join(",")),` in contestreader_yaml.rs
    let required_contests = contest.requires_contest.as_ref()?.split(',');

    for req_contest in required_contests {
        if conn.has_participation_by_contest_file(session.id, &contest.location, req_contest) {
            return Some(true);
        }
    }

    Some(false)
}

fn check_contest_constraints(session: &SessionUser, contest: &Contest) -> ContestStartConstraints {
    let now = time::get_time();
    let student_grade = session.grade % 100 - if session.grade / 100 == 1 { 1 } else { 0 };

    let contest_not_begun = contest.start.map(|start| now < start).unwrap_or(false);
    let contest_over = contest.end.map(|end| now > end).unwrap_or(false);
    let grade_too_low =
        contest.min_grade.map(|min_grade| student_grade < min_grade && !session.is_teacher).unwrap_or(false);
    let grade_too_high =
        contest.max_grade.map(|max_grade| student_grade > max_grade && !session.is_teacher).unwrap_or(false);

    let contest_running = !contest_not_begun && !contest_over;
    let grade_matching = !grade_too_low && !grade_too_high;

    ContestStartConstraints { contest_not_begun,
                              contest_over,
                              contest_running,
                              grade_too_low,
                              grade_too_high,
                              grade_matching }
}

#[derive(Serialize, Deserialize)]
pub struct ContestTimeInfo {
    pub passed_secs_total: i64,
    pub left_secs_total: i64,
    pub left_mins_total: i64,
    pub left_hour: i64,
    pub left_min: i64,
    pub left_sec: i64,
    pub has_timelimit: bool,
    pub is_time_left: bool,
    pub exempt_from_timelimit: bool,
    pub can_still_compete: bool,
    pub review_has_timelimit: bool,
    pub has_future_review: bool,
    pub has_review_end: bool,
    pub is_review: bool,
    pub can_still_compete_or_review: bool,

    pub until_review_start_day: i64,
    pub until_review_start_hour: i64,
    pub until_review_start_min: i64,

    pub until_review_end_day: i64,
    pub until_review_end_hour: i64,
    pub until_review_end_min: i64,
}

fn check_contest_time_left(session: &SessionUser, contest: &Contest, participation: &Participation) -> ContestTimeInfo {
    let now = time::get_time();
    let passed_secs_total = now.sec - participation.start.sec;
    if passed_secs_total < 0 {
        // Handle inconsistent server time
    }
    let left_secs_total = i64::from(contest.duration) * 60 - passed_secs_total;

    let is_time_left = contest.duration == 0 || left_secs_total >= 0;
    let exempt_from_timelimit = session.is_teacher() || session.is_admin();

    let can_still_compete = is_time_left || exempt_from_timelimit;

    let review_has_timelimit = contest.review_end.is_none() && contest.review_start.is_some();
    let has_future_review = (contest.review_start.is_some() || contest.review_end.is_some())
                            && contest.review_end.map(|end| end > now).unwrap_or(true);
    let has_review_end = contest.review_end.is_some();
    let is_review = !can_still_compete
                    && (contest.review_start.is_some() || contest.review_end.is_some())
                    && contest.review_start.map(|start| now >= start).unwrap_or(true)
                    && contest.review_end.map(|end| now <= end).unwrap_or(true);

    let until_review_start = contest.review_start.map(|start| start.sec - now.sec).unwrap_or(0);
    let until_review_end = contest.review_end.map(|end| end.sec - now.sec).unwrap_or(0);

    ContestTimeInfo { passed_secs_total,
                      left_secs_total,
                      left_mins_total: left_secs_total / 60,
                      left_hour: left_secs_total / (60 * 60),
                      left_min: (left_secs_total / 60) % 60,
                      left_sec: left_secs_total % 60,
                      has_timelimit: contest.duration != 0,
                      is_time_left,
                      exempt_from_timelimit,
                      can_still_compete,
                      review_has_timelimit,
                      has_future_review,
                      has_review_end,
                      is_review,
                      can_still_compete_or_review: can_still_compete || is_review,

                      until_review_start_day: until_review_start / (60 * 60 * 24),
                      until_review_start_hour: (until_review_start / (60 * 60)) % 24,
                      until_review_start_min: (until_review_start / 60) % 60,

                      until_review_end_day: until_review_end / (60 * 60 * 24),
                      until_review_end_hour: (until_review_end / (60 * 60)) % 24,
                      until_review_end_min: (until_review_end / 60) % 60 }
}

pub fn show_contest<T: MedalConnection>(conn: &T, contest_id: i32, session_token: &str,
                                        query_string: Option<String>, login_info: LoginInfo, secret: Option<String>)
                                        -> MedalResult<Result<MedalValue, i32>> {
    let session = conn.get_session_or_new(&session_token).unwrap();

    if session.logincode.is_some() && session.firstname.is_none() {
        return Err(MedalError::AccountIncomplete);
    }

    let contest = conn.get_contest_by_id_complete(contest_id).ok_or(MedalError::UnknownId)?;
    let grades = conn.get_contest_user_grades(&session_token, contest_id);

    let mut opt_part = conn.get_participation(session.id, contest_id);

    let ci = ContestInfo { id: contest.id.unwrap(),
                           name: contest.name.clone(),
                           duration: contest.duration,
                           public: contest.public,
                           requires_login: contest.requires_login.unwrap_or(false),
                           image: None,
                           language: None,
                           tags: Vec::new() };

    let mut data = json_val::Map::new();
    data.insert("parent".to_string(), to_json(&"base"));
    data.insert("empty".to_string(), to_json(&"empty"));
    data.insert("contest".to_string(), to_json(&ci));
    data.insert("title".to_string(), to_json(&ci.name));
    data.insert("message".to_string(), to_json(&contest.message));
    fill_oauth_data(login_info, &mut data);

    if secret.is_some() && secret != contest.secret {
        return Err(MedalError::AccessDenied);
    }

    let has_secret = contest.secret.is_some();
    let mut require_secret = false;
    if has_secret {
        data.insert("secret_field".to_string(), to_json(&true));

        if secret.is_some() {
            data.insert("secret_field_prefill".to_string(), to_json(&secret));
        } else {
            require_secret = true;
        }
    }

    let constraints = check_contest_constraints(&session, &contest);
    let is_qualified = check_contest_qualification(conn, &session, &contest).unwrap_or(true);

    let has_tasks = contest.taskgroups.len() > 0;
    let can_start = constraints.contest_running
                    && constraints.grade_matching
                    && is_qualified
                    && (has_tasks || has_secret)
                    && (session.is_logged_in() || contest.secret.is_some() && !contest.requires_login.unwrap_or(false));

    let has_duration = contest.duration > 0;

    data.insert("constraints".to_string(), to_json(&constraints));
    data.insert("is_qualified".to_string(), to_json(&is_qualified));
    data.insert("has_duration".to_string(), to_json(&has_duration));
    data.insert("can_start".to_string(), to_json(&can_start));
    data.insert("has_tasks".to_string(), to_json(&has_tasks));
    data.insert("no_tasks".to_string(), to_json(&!has_tasks));

    // Autostart if appropriate
    // TODO: Should participation start automatically for teacher? Even before the contest start?
    // Should teachers have all time access or only the same limited amount of time?
    // if opt_part.is_none() && (contest.duration == 0 || session.is_teacher) {
    if opt_part.is_none()
       && contest.duration == 0
       && constraints.contest_running
       && constraints.grade_matching
       && !require_secret
       && contest.requires_login != Some(true)
    {
        conn.new_participation(&session_token, contest_id).map_err(|_| MedalError::AccessDenied)?;
        opt_part = Some(Participation { contest: contest_id, user: session.id, start: time::get_time() });
    }

    let now = time::get_time();
    if let Some(start) = contest.start {
        if now < start {
            let until = start - now;
            data.insert("time_until_start".to_string(),
                        to_json(&[until.num_days(), until.num_hours() % 24, until.num_minutes() % 60]));
        }
    }

    if let Some(end) = contest.end {
        if now < end {
            let until = end - now;
            data.insert("time_until_end".to_string(),
                        to_json(&[until.num_days(), until.num_hours() % 24, until.num_minutes() % 60]));
        }
    }

    if session.is_logged_in() {
        data.insert("logged_in".to_string(), to_json(&true));
        data.insert("username".to_string(), to_json(&session.username));
        data.insert("firstname".to_string(), to_json(&session.firstname));
        data.insert("lastname".to_string(), to_json(&session.lastname));
        data.insert("teacher".to_string(), to_json(&session.is_teacher));
        data.insert("csrf_token".to_string(), to_json(&session.csrf_token));
    }

    if let Some(participation) = opt_part {
        let time_info = check_contest_time_left(&session, &contest, &participation);
        data.insert("time_info".to_string(), to_json(&time_info));

        let time_left_formatted =
            format!("{}:{:02}:{:02}", time_info.left_hour, time_info.left_min, time_info.left_sec);
        data.insert("time_left_formatted".to_string(), to_json(&time_left_formatted));

        let mut totalgrade = 0;
        let mut max_totalgrade = 0;

        let mut tasks = Vec::new();
        for (taskgroup, grade) in contest.taskgroups.into_iter().zip(grades) {
            let subtaskstars = generate_subtaskstars(&taskgroup, &grade, None);
            let ti = TaskInfo { name: taskgroup.name, subtasks: subtaskstars };
            tasks.push(ti);

            totalgrade += grade.grade.unwrap_or(0);
            max_totalgrade += taskgroup.tasks.iter().map(|x| x.stars).max().unwrap_or(0);
        }
        let relative_points = if max_totalgrade > 0 { (totalgrade * 100) / max_totalgrade } else { 0 };

        data.insert("tasks".to_string(), to_json(&tasks));

        data.insert("is_started".to_string(), to_json(&true));
        data.insert("total_points".to_string(), to_json(&totalgrade));
        data.insert("max_total_points".to_string(), to_json(&max_totalgrade));
        data.insert("relative_points".to_string(), to_json(&relative_points));
        data.insert("lean_page".to_string(), to_json(&true));

        if has_tasks && contest.standalone_task.unwrap_or(false) {
            return Ok(Err(tasks[0].subtasks[0].id));
        }
    }

    // This only checks if a query string is existent, so any query string will
    // lead to the assumption that a bare page is requested. This is useful to
    // disable caching (via random token) but should be changed if query string
    // can obtain more than only this meaning in the future
    if query_string.is_none() {
        data.insert("not_bare".to_string(), to_json(&true));
    }

    Ok(Ok(("contest".to_owned(), data)))
}

pub fn show_contest_results<T: MedalConnection>(conn: &T, contest_id: i32, session_token: &str) -> MedalValueResult {
    let session = conn.get_session(&session_token).ensure_logged_in().ok_or(MedalError::NotLoggedIn)?;
    let mut data = json_val::Map::new();
    fill_user_data(&session, &mut data);

    let (tasknames, resultdata) = conn.get_contest_groups_grades(session.id, contest_id);

    #[derive(Serialize, Deserialize)]
    struct UserResults {
        firstname: String,
        lastname: String,
        user_id: i32,
        grade: String,
        logincode: String,
        annotation: String,
        results: Vec<String>,
    }

    #[derive(Serialize, Deserialize)]
    struct GroupResults {
        groupname: String,
        group_id: i32,
        groupcode: String,
        user_results: Vec<UserResults>,
    }

    let mut results: Vec<GroupResults> = Vec::new();
    let mut has_annotations = false;

    for (group, groupdata) in resultdata {
        let mut groupresults: Vec<UserResults> = Vec::new();

        for (user, userdata) in groupdata {
            let mut userresults: Vec<String> = Vec::new();

            userresults.push(String::new());
            let mut summe = 0;

            for grade in userdata {
                if let Some(g) = grade.grade {
                    userresults.push(format!("{}", g));
                    summe += g;
                } else {
                    userresults.push("–".to_string());
                }
            }

            userresults[0] = format!("{}", summe);

            if user.annotation.is_some() {
                has_annotations = true;
            }

            groupresults.push(UserResults { firstname: user.firstname.unwrap_or_else(|| "–".to_string()),
                                            lastname: user.lastname.unwrap_or_else(|| "–".to_string()),
                                            user_id: user.id,
                                            grade: grade_to_string(user.grade),
                                            logincode: user.logincode.unwrap_or_else(|| "".to_string()),
                                            annotation: user.annotation.unwrap_or_else(|| "".to_string()),
                                            results: userresults });
        }

        results.push(GroupResults { groupname: group.name.to_string(),
                                    group_id: group.id.unwrap_or(0),
                                    groupcode: group.groupcode,
                                    user_results: groupresults });
    }

    data.insert("taskname".to_string(), to_json(&tasknames));
    data.insert("result".to_string(), to_json(&results));
    data.insert("has_annotations".to_string(), to_json(&has_annotations));

    let c = conn.get_contest_by_id(contest_id).ok_or(MedalError::UnknownId)?;
    let ci = ContestInfo { id: c.id.unwrap(),
                           name: c.name.clone(),
                           duration: c.duration,
                           public: c.public,
                           requires_login: c.requires_login.unwrap_or(false),
                           image: None,
                           language: None,
                           tags: Vec::new() };

    data.insert("contest".to_string(), to_json(&ci));
    data.insert("contestname".to_string(), to_json(&c.name));

    Ok(("contestresults".to_owned(), data))
}

pub fn start_contest<T: MedalConnection>(conn: &T, contest_id: i32, session_token: &str, csrf_token: &str,
                                         secret: Option<String>)
                                         -> MedalResult<()> {
    // TODO: Is _or_new the right semantic? We need a CSRF token anyway …
    let session = conn.get_session_or_new(&session_token).unwrap();
    let contest = conn.get_contest_by_id(contest_id).ok_or(MedalError::UnknownId)?;

    // Check logged in or open contest
    if contest.duration != 0
       && !session.is_logged_in()
       && (contest.requires_login.unwrap_or(false) || contest.secret.is_none())
    {
        return Err(MedalError::AccessDenied);
    }

    // Check CSRF token
    if session.is_logged_in() && session.csrf_token != csrf_token {
        return Err(MedalError::CsrfCheckFailed);
    }

    // Check other constraints
    let constraints = check_contest_constraints(&session, &contest);

    if !(constraints.contest_running && constraints.grade_matching) {
        return Err(MedalError::AccessDenied);
    }

    let is_qualified = check_contest_qualification(conn, &session, &contest);

    if is_qualified == Some(false) {
        return Err(MedalError::AccessDenied);
    }

    if contest.secret != secret {
        return Err(MedalError::AccessDenied);
    }

    // Start contest
    match conn.new_participation(&session_token, contest_id) {
        Ok(_) => Ok(()),
        _ => Err(MedalError::AccessDenied), // Contest already started TODO: Maybe redirect to page with hint
    }
}

pub fn login<T: MedalConnection>(conn: &T, login_data: (String, String), login_info: LoginInfo)
                                 -> Result<String, MedalValue> {
    let (username, password) = login_data;

    match conn.login(None, &username, &password) {
        Ok(session_token) => Ok(session_token),
        Err(()) => {
            let mut data = json_val::Map::new();
            data.insert("reason".to_string(), to_json(&"Login fehlgeschlagen. Bitte erneut versuchen.".to_string()));
            data.insert("username".to_string(), to_json(&username));
            data.insert("parent".to_string(), to_json(&"base"));

            fill_oauth_data(login_info, &mut data);

            Err(("login".to_owned(), data))
        }
    }
}

pub fn login_with_code<T: MedalConnection>(
    conn: &T, code: &str, login_info: LoginInfo)
    -> Result<Result<String, String>, (String, json_val::Map<String, json_val::Value>)> {
    match conn.login_with_code(None, &code.trim()) {
        Ok(session_token) => Ok(Ok(session_token)),
        Err(()) => match conn.create_user_with_groupcode(None, &code.trim()) {
            Ok(session_token) => Ok(Err(session_token)),
            Err(()) => {
                let mut data = json_val::Map::new();
                data.insert("reason".to_string(), to_json(&"Kein gültiger Code. Bitte erneut versuchen.".to_string()));
                data.insert("code".to_string(), to_json(&code));
                data.insert("parent".to_string(), to_json(&"base"));

                fill_oauth_data(login_info, &mut data);

                Err(("login".to_owned(), data))
            }
        },
    }
}

pub fn logout<T: MedalConnection>(conn: &T, session_token: Option<String>) {
    session_token.map(|token| conn.logout(&token));
}

#[cfg(feature = "signup")]
pub fn signup<T: MedalConnection>(conn: &T, session_token: Option<String>, signup_data: (String, String, String))
                                  -> MedalResult<SignupResult> {
    let (username, email, password) = signup_data;

    if username == "" || email == "" || password == "" {
        return Ok(SignupResult::EmptyFields);
    }

    let salt = helpers::make_salt();
    let hash = helpers::hash_password(&password, &salt)?;

    let result = conn.signup(&session_token.unwrap(), &username, &email, hash, &salt);
    Ok(result)
}

#[cfg(feature = "signup")]
pub fn signupdata(query_string: Option<String>) -> json_val::Map<String, json_val::Value> {
    let mut data = json_val::Map::new();
    if let Some(query) = query_string {
        if let Some(status) = query.strip_prefix("status=") {
            if ["EmailTaken", "UsernameTaken", "UserLoggedIn", "EmptyFields"].contains(&status) {
                data.insert((status).to_string(), to_json(&true));
            }
        }
    }
    data
}

pub fn load_submission<T: MedalConnection>(conn: &T, task_id: i32, session_token: &str, subtask: Option<String>,
                                           submission_id: Option<i32>)
                                           -> MedalResult<String> {
    let session = conn.get_session(&session_token).ensure_alive().ok_or(MedalError::NotLoggedIn)?;

    match submission_id {
        None => match conn.load_submission(&session, task_id, subtask.as_deref()) {
            Some(submission) => Ok(submission.value),
            None => Ok("{}".to_string()),
        },
        Some(submission_id) => {
            let (submission, _, _, _) =
                conn.get_submission_by_id_complete_shallow_contest(submission_id).ok_or(MedalError::UnknownId)?;

            // Is it not our own submission?
            if submission.user != session.id && !session.is_admin.unwrap_or(false) {
                if let Some((_, Some(group))) = conn.get_user_and_group_by_id(submission.user) {
                    if group.admin != session.id {
                        // We are not admin of the user's group
                        return Err(MedalError::AccessDenied);
                    }
                } else {
                    // The user has no group
                    return Err(MedalError::AccessDenied);
                }
            }
            Ok(submission.value)
        }
    }
}

pub fn save_submission<T: MedalConnection>(conn: &T, task_id: i32, session_token: &str, csrf_token: &str,
                                           data: String, grade_percentage: i32, subtask: Option<String>)
                                           -> MedalResult<String> {
    let session = conn.get_session(&session_token).ensure_alive().ok_or(MedalError::NotLoggedIn)?;

    if session.csrf_token != csrf_token {
        return Err(MedalError::CsrfCheckFailed);
    }

    let (t, _, contest) = conn.get_task_by_id_complete(task_id).ok_or(MedalError::UnknownId)?;

    match conn.get_participation(session.id, contest.id.expect("Value from database")) {
        None => return Err(MedalError::AccessDenied),
        Some(participation) => {
            let time_info = check_contest_time_left(&session, &contest, &participation);
            if !time_info.can_still_compete && time_info.left_secs_total < -10 {
                return Err(MedalError::AccessDenied);
                // Contest over
                // TODO: Nicer message!
            }
        }
    }

    /* Here, two variants of the grade are calculated. Which one is correct depends on how the percentage value is
     * calculated in the task. Currently, grade_rounded is the correct one, but if that ever changes, the other code
     * can just be used.
     *
     * Switch to grade_truncated, when a user scores 98/99 but only gets 97/99 awarded.
     * Switch to grade_rounded, when a user scores 5/7 but only gets 4/7 awarded.
     */

    /* Code for percentages calculated with integer rounding.
     *
     * This is a poor man's rounding that only works for division by 100.
     *
     *   floor((floor((x*10)/100)+5)/10) = round(x/100)
     */
    let grade_rounded = ((grade_percentage * t.stars * 10) / 100 + 5) / 10;

    /* Code for percentages calculated with integer truncation.
     *
     * Why add one to grade_percentage and divide by 101?
     *
     * For all m in 1..100 and all n in 0..n, this holds:
     *
     *   floor( ((floor(n / m * 100)+1) * m ) / 101 ) = n
     *
     * Thus, when percentages are calculated as
     *
     *   p = floor(n / m * 100)
     *
     * we can recover n by using
     *
     *   n = floor( ((p+1) * m) / 101 )
     */
    // let grade_truncated = ((grade_percentage+1) * t.stars) / 101;

    let submission = Submission { id: None,
                                  user: session.id,
                                  task: task_id,
                                  grade: grade_rounded,
                                  validated: false,
                                  nonvalidated_grade: grade_rounded,
                                  needs_validation: true,
                                  subtask_identifier: subtask,
                                  value: data,
                                  date: time::get_time() };

    conn.submit_submission(submission);

    Ok("{}".to_string())
}

pub fn show_task<T: MedalConnection>(conn: &T, task_id: i32, session_token: &str, autosaveinterval: u64)
                                     -> MedalResult<Result<MedalValue, i32>> {
    let session = conn.get_session_or_new(&session_token).unwrap();

    let (t, tg, contest) = conn.get_task_by_id_complete(task_id).ok_or(MedalError::UnknownId)?;
    let grade = conn.get_taskgroup_user_grade(&session_token, tg.id.unwrap()); // TODO: Unwrap?
    let tasklist = conn.get_contest_by_id_complete(contest.id.unwrap()).ok_or(MedalError::UnknownId)?; // TODO: Unwrap?

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

    match conn.get_own_participation(&session_token, contest.id.expect("Value from database")) {
        None => Ok(Err(contest.id.unwrap())),
        Some(participation) => {
            let mut data = json_val::Map::new();
            data.insert("subtasks".to_string(), to_json(&subtaskstars));
            data.insert("prevtask".to_string(), to_json(&prevtaskgroup.map(|tg| tg.tasks[0].id)));
            data.insert("nexttask".to_string(), to_json(&nexttaskgroup.map(|tg| tg.tasks[0].id))); // TODO: fail better

            let time_info = check_contest_time_left(&session, &contest, &participation);
            data.insert("time_info".to_string(), to_json(&time_info));

            data.insert("time_left_mh_formatted".to_string(),
                        to_json(&format!("{}:{:02}", time_info.left_hour, time_info.left_min)));
            data.insert("time_left_sec_formatted".to_string(), to_json(&format!(":{:02}", time_info.left_sec)));

            let auto_save_interval_ms = if autosaveinterval > 0 && autosaveinterval < 31536000000 {
                autosaveinterval * 1000
            } else {
                31536000000
            };
            data.insert("auto_save_interval_ms".to_string(), to_json(&auto_save_interval_ms));

            if time_info.can_still_compete || time_info.is_review {
                data.insert("contestname".to_string(), to_json(&contest.name));
                data.insert("name".to_string(), to_json(&tg.name));
                data.insert("title".to_string(), to_json(&format!("Aufgabe „{}“ in {}", &tg.name, &contest.name)));
                data.insert("taskid".to_string(), to_json(&task_id));
                data.insert("csrf_token".to_string(), to_json(&session.csrf_token));
                data.insert("contestid".to_string(), to_json(&contest.id));
                data.insert("readonly".to_string(), to_json(&time_info.is_review));

                let (template, tasklocation) = if let Some(language) = t.language {
                    match language.as_str() {
                        "blockly" => ("wtask".to_owned(), t.location.as_str()),
                        "python" => {
                            data.insert("tasklang".to_string(), to_json(&"python"));
                            ("wtask".to_owned(), t.location.as_str())
                        }
                        _ => ("task".to_owned(), t.location.as_str()),
                    }
                } else {
                    match t.location.chars().next() {
                        Some('B') => ("wtask".to_owned(), &t.location[1..]),
                        Some('P') => {
                            data.insert("tasklang".to_string(), to_json(&"python"));
                            ("wtask".to_owned(), &t.location[1..])
                        }
                        _ => ("task".to_owned(), t.location.as_str()),
                    }
                };

                let taskpath = format!("{}{}", contest.location, &tasklocation);
                data.insert("taskpath".to_string(), to_json(&taskpath));

                Ok(Ok((template, data)))
            } else {
                // Contest over
                Ok(Err(contest.id.unwrap()))
            }
        }
    }
}

pub fn review_task<T: MedalConnection>(conn: &T, task_id: i32, session_token: &str, submission_id: i32)
                                       -> MedalResult<Result<MedalValue, i32>> {
    let session = conn.get_session_or_new(&session_token).unwrap();

    let (submission, t, tg, contest) =
        conn.get_submission_by_id_complete_shallow_contest(submission_id).ok_or(MedalError::UnknownId)?;

    // TODO: We make a fake grade here, that represents this very submission, but maybe it is more sensible to retrieve
    // the actual grade here? If yes, use conn.get_taskgroup_user_grade(&session_token, tg.id.unwrap());
    let grade = Grade { taskgroup: tg.id.unwrap(),
                        user: session.id,
                        grade: Some(submission.grade),
                        validated: submission.validated };

    // Is it not our own submission?
    if submission.user != session.id && !session.is_admin.unwrap_or(false) {
        if let Some((_, Some(group))) = conn.get_user_and_group_by_id(submission.user) {
            if group.admin != session.id {
                // We are not admin of the user's group
                return Err(MedalError::AccessDenied);
            }
        } else {
            // The user has no group
            return Err(MedalError::AccessDenied);
        }
    }

    let subtaskstars = generate_subtaskstars(&tg, &grade, Some(task_id)); // TODO does this work in general?

    let mut data = json_val::Map::new();
    data.insert("subtasks".to_string(), to_json(&subtaskstars));

    let time_info = ContestTimeInfo { passed_secs_total: 0,
                                      left_secs_total: 0,
                                      left_mins_total: 0,
                                      left_hour: 0,
                                      left_min: 0,
                                      left_sec: 0,
                                      has_timelimit: contest.duration != 0,
                                      is_time_left: false,
                                      exempt_from_timelimit: true,
                                      can_still_compete: false,
                                      review_has_timelimit: false,
                                      has_future_review: false,
                                      has_review_end: false,
                                      is_review: true,
                                      can_still_compete_or_review: true,

                                      until_review_start_day: 0,
                                      until_review_start_hour: 0,
                                      until_review_start_min: 0,

                                      until_review_end_day: 0,
                                      until_review_end_hour: 0,
                                      until_review_end_min: 0 };

    data.insert("time_info".to_string(), to_json(&time_info));

    data.insert("time_left_mh_formatted".to_string(),
                to_json(&format!("{}:{:02}", time_info.left_hour, time_info.left_min)));
    data.insert("time_left_sec_formatted".to_string(), to_json(&format!(":{:02}", time_info.left_sec)));

    data.insert("auto_save_interval_ms".to_string(), to_json(&0));

    //data.insert("contestname".to_string(), to_json(&contest.name));
    data.insert("name".to_string(), to_json(&tg.name));
    data.insert("title".to_string(), to_json(&format!("Aufgabe „{}“ in {}", &tg.name, &contest.name)));
    data.insert("taskid".to_string(), to_json(&task_id));
    data.insert("csrf_token".to_string(), to_json(&session.csrf_token));
    //data.insert("contestid".to_string(), to_json(&contest.id));
    data.insert("readonly".to_string(), to_json(&time_info.is_review));

    data.insert("submission".to_string(), to_json(&submission_id));

    let (template, tasklocation) = match t.location.chars().next() {
        Some('B') => ("wtask".to_owned(), &t.location[1..]),
        Some('P') => {
            data.insert("tasklang".to_string(), to_json(&"python"));
            ("wtask".to_owned(), &t.location[1..])
        }
        _ => ("task".to_owned(), &t.location as &str),
    };

    let taskpath = format!("{}{}", contest.location, &tasklocation);
    data.insert("taskpath".to_string(), to_json(&taskpath));

    Ok(Ok((template, data)))
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
    pub sex: String,
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

    let v: Vec<MemberInfo> = group.members
                                  .iter()
                                  .filter_map(|m| {
                                      Some(MemberInfo { id: m.id,
                                                        firstname: m.firstname.clone()?,
                                                        lastname: m.lastname.clone()?,
                                                        sex: (match m.sex {
                                                                 Some(0) | None => "/",
                                                                 Some(1) => "m",
                                                                 Some(2) => "w",
                                                                 Some(3) => "d",
                                                                 Some(4) => "…",
                                                                 _ => "?",
                                                             }).to_string(),
                                                        grade: grade_to_string(m.grade),
                                                        logincode: m.logincode.clone()? })
                                  })
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
    let session = conn.get_session(&session_token)
                      .ensure_logged_in()
                      .ok_or(MedalError::AccessDenied)?
                      .ensure_teacher_or_admin()
                      .ok_or(MedalError::AccessDenied)?;

    if session.csrf_token != csrf_token {
        return Err(MedalError::CsrfCheckFailed);
    }

    let mut groupcode = String::new();
    for i in 0..10 {
        if i == 9 {
            panic!("ERROR: Too many groupcode collisions! Give up ...");
        }
        groupcode = helpers::make_groupcode();
        if !conn.code_exists(&groupcode) {
            break;
        }
        println!("WARNING: Groupcode collision! Retrying ...");
    }

    let mut group = Group { id: None, name, groupcode, tag, admin: session.id, members: Vec::new() };

    conn.add_group(&mut group);

    Ok(group.id.unwrap())
}

pub fn group_csv<T: MedalConnection>(conn: &T, session_token: &str, sex_infos: SexInformation) -> MedalValueResult {
    let session = conn.get_session(&session_token).ensure_logged_in().ok_or(MedalError::NotLoggedIn)?;

    let mut data = json_val::Map::new();
    data.insert("csrf_token".to_string(), to_json(&session.csrf_token));

    data.insert("require_sex".to_string(), to_json(&sex_infos.require_sex));
    data.insert("allow_sex_na".to_string(), to_json(&sex_infos.allow_sex_na));
    data.insert("allow_sex_diverse".to_string(), to_json(&sex_infos.allow_sex_diverse));
    data.insert("allow_sex_other".to_string(), to_json(&sex_infos.allow_sex_other));

    Ok(("groupcsv".to_string(), data))
}

// TODO: Should creating the users and groups happen in a batch operation to speed things up?
pub fn upload_groups<T: MedalConnection>(conn: &T, session_token: &str, csrf_token: &str, group_data: &str)
                                         -> MedalResult<()> {
    let session = conn.get_session(&session_token).ensure_logged_in().ok_or(MedalError::NotLoggedIn)?;

    if session.csrf_token != csrf_token {
        return Err(MedalError::CsrfCheckFailed);
    }

    let mut v: Vec<Vec<String>> = serde_json::from_str(group_data).or(Err(MedalError::AccessDenied))?; // TODO: Change error type
    v.sort_unstable_by(|a, b| a[0].partial_cmp(&b[0]).unwrap());

    let mut groupcode = String::new();
    let mut name = String::new();
    let mut group =
        Group { id: None, name: name.clone(), groupcode, tag: String::new(), admin: session.id, members: Vec::new() };

    for line in v {
        if name != line[0] {
            if name != "" {
                conn.create_group_with_users(group);
            }
            name = line[0].clone();

            groupcode = String::new();
            for i in 0..10 {
                if i == 9 {
                    panic!("ERROR: Too many groupcode collisions! Give up ...");
                }
                groupcode = helpers::make_groupcode();
                if !conn.code_exists(&groupcode) {
                    break;
                }
                println!("WARNING: Groupcode collision! Retrying ...");
            }

            group = Group { id: None,
                            name: name.clone(),
                            groupcode,
                            tag: name.clone(),
                            admin: session.id,
                            members: Vec::new() };
        }

        let mut user = SessionUser::group_user_stub();
        user.grade = line[1].parse::<i32>().unwrap_or(0);
        user.firstname = Some(line[2].clone());
        user.lastname = Some(line[3].clone());

        use db_objects::Sex;
        match line[4].as_str() {
            "m" => user.sex = Some(Sex::Male as i32),
            "f" => user.sex = Some(Sex::Female as i32),
            "d" => user.sex = Some(Sex::Diverse as i32),
            _ => user.sex = None,
        }

        group.members.push(user);
    }
    conn.create_group_with_users(group);

    Ok(())
}

pub fn contest_admission_csv<T: MedalConnection>(conn: &T, session_token: &str) -> MedalValueResult {
    let session = conn.get_session(&session_token).ensure_logged_in().ok_or(MedalError::NotLoggedIn)?;

    let mut data = json_val::Map::new();
    data.insert("csrf_token".to_string(), to_json(&session.csrf_token));

    Ok(("admin_admissioncsv".to_string(), data))
}

pub fn upload_contest_admission_csv<T: MedalConnection>(conn: &T, session_token: &str, csrf_token: &str,
                                                        contest_id: i32, admission_data: &str)
                                                        -> MedalResult<()> {
    let session = conn.get_session(&session_token).ensure_logged_in().ok_or(MedalError::NotLoggedIn)?;

    if session.csrf_token != csrf_token {
        return Err(MedalError::CsrfCheckFailed);
    }

    let v: Vec<Vec<String>> = serde_json::from_str(admission_data).or(Err(MedalError::AccessDenied))?; // TODO: Change error type

    let w: Vec<(i32, Option<String>)> =
        v.into_iter()
         .map(|vv| (vv[0].parse().unwrap_or(-1), if vv[1].len() == 0 { None } else { Some(vv[1].clone()) }))
         .collect();

    let _annotations_inserted = conn.insert_contest_annotations(contest_id, w);

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

pub struct SexInformation {
    pub require_sex: bool,
    pub allow_sex_na: bool,
    pub allow_sex_diverse: bool,
    pub allow_sex_other: bool,
}

pub fn show_profile<T: MedalConnection>(conn: &T, session_token: &str, user_id: Option<i32>,
                                        query_string: Option<String>, sex_infos: SexInformation)
                                        -> MedalValueResult {
    let session = conn.get_session(&session_token).ensure_logged_in().ok_or(MedalError::NotLoggedIn)?;

    let mut data = json_val::Map::new();
    fill_user_data(&session, &mut data);

    data.insert("require_sex".to_string(), to_json(&sex_infos.require_sex));
    data.insert("allow_sex_na".to_string(), to_json(&sex_infos.allow_sex_na));
    data.insert("allow_sex_diverse".to_string(), to_json(&sex_infos.allow_sex_diverse));
    data.insert("allow_sex_other".to_string(), to_json(&sex_infos.allow_sex_other));

    match user_id {
        None => {
            data.insert("profile_firstname".to_string(), to_json(&session.firstname));
            data.insert("profile_lastname".to_string(), to_json(&session.lastname));
            data.insert("profile_street".to_string(), to_json(&session.street));
            data.insert("profile_zip".to_string(), to_json(&session.zip));
            data.insert("profile_city".to_string(), to_json(&session.city));
            data.insert(format!("sel{}", session.grade), to_json(&"selected"));
            if let Some(sex) = session.sex {
                data.insert(format!("sex_{}", sex), to_json(&"selected"));
            } else {
                data.insert("sex_None".to_string(), to_json(&"selected"));
            }

            data.insert("profile_logincode".to_string(), to_json(&session.logincode));
            if session.password.is_some() {
                data.insert("profile_username".to_string(), to_json(&session.username));
            }
            if session.managed_by.is_none() {
                data.insert("profile_not_in_group".into(), to_json(&true));
            }
            if session.oauth_provider != Some("pms".to_string()) {
                data.insert("profile_not_pms".into(), to_json(&true));
                // This should be changed so that it can be configured if
                // addresses can be obtained from OAuth provider
            }
            data.insert("ownprofile".into(), to_json(&true));

            if let Some(query) = query_string {
                if let Some(status) = query.strip_prefix("status=") {
                    if ["NothingChanged",
                        "DataChanged",
                        "PasswordChanged",
                        "PasswordMissmatch",
                        "firstlogin",
                        "SignedUp"].contains(&status)
                    {
                        data.insert((status).to_string(), to_json(&true));
                    }
                }
            }

            let now = time::get_time();

            // TODO: Needs to be filtered
            let participations: (Vec<(i32, String, bool, bool, bool)>, Vec<(i32, String, bool, bool, bool)>) =
                conn.get_all_participations_complete(session.id)
                    .into_iter()
                    .rev()
                    .map(|(participation, contest)| {
                        let passed_secs = now.sec - participation.start.sec;
                        let left_secs = i64::from(contest.duration) * 60 - passed_secs;
                        let is_time_left = contest.duration == 0 || left_secs >= 0;
                        let has_timelimit = contest.duration != 0;
                        let requires_login = contest.requires_login == Some(true);
                        (contest.id.unwrap(), contest.name, has_timelimit, is_time_left, requires_login)
                    })
                    .partition(|contest| contest.2 && !contest.4);
            data.insert("participations".into(), to_json(&participations));

            let stars_count = conn.count_all_stars(session.id);
            data.insert("stars_count".into(), to_json(&stars_count));
            let stars_message = match stars_count {
                                    0 => "Auf gehts, dein erster Stern wartet auf dich!",
                                    1..=9 => "Ein hervorragender Anfang!",
                                    10..=99 => "Das ist ziemlich gut!",
                                    100..=999 => "Ein wahrer Meister!",
                                    _ => "Wow! Einfach wow!",
                                }.to_string();

            data.insert("stars_message".into(), to_json(&stars_message));
        }
        // Case user_id: teacher modifing a students profile
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
            if let Some(sex) = user.sex {
                data.insert(format!("sex_{}", sex), to_json(&"selected"));
            } else {
                data.insert("sex_None".to_string(), to_json(&"selected"));
            }

            data.insert("profile_logincode".to_string(), to_json(&user.logincode));
            if user.username.is_some() {
                data.insert("profile_username".to_string(), to_json(&user.username));
            }
            if user.managed_by.is_none() {
                data.insert("profile_not_in_group".into(), to_json(&true));
            }
            if session.oauth_provider != Some("pms".to_string()) {
                data.insert("profile_not_pms".into(), to_json(&true));
            }
            data.insert("ownprofile".into(), to_json(&false));

            if let Some(query) = query_string {
                if let Some(status) = query.strip_prefix("status=") {
                    if ["NothingChanged", "DataChanged", "PasswordChanged", "PasswordMissmatch"].contains(&status) {
                        data.insert((status).to_string(), to_json(&true));
                    }
                }
            }
        }
    }

    Ok(("profile".to_string(), data))
}

#[derive(Debug, PartialEq, Eq)]
pub enum ProfileStatus {
    NothingChanged,
    DataChanged,
    PasswordChanged,
    PasswordMissmatch,
}
impl From<ProfileStatus> for String {
    fn from(s: ProfileStatus) -> String { format!("{:?}", s) }
}

pub fn edit_profile<T: MedalConnection>(conn: &T, session_token: &str, user_id: Option<i32>, csrf_token: &str,
                                        (firstname,
                                         lastname,
                                         street,
                                         zip,
                                         city,
                                         password,
                                         password_repeat,
                                         grade,
                                         sex): (String,
                                         String,
                                         Option<String>,
                                         Option<String>,
                                         Option<String>,
                                         Option<String>,
                                         Option<String>,
                                         i32,
                                         Option<i32>))
                                        -> MedalResult<ProfileStatus> {
    let mut session = conn.get_session(&session_token).ensure_logged_in().ok_or(MedalError::NotLoggedIn)?;

    if session.csrf_token != csrf_token {
        return Err(MedalError::AccessDenied); // CsrfError
    }

    let mut result = ProfileStatus::NothingChanged;

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
        }
    }

    if result == ProfileStatus::NothingChanged {
        if session.firstname.as_ref() == Some(&firstname)
           && session.lastname.as_ref() == Some(&lastname)
           && session.street == street
           && session.zip == zip
           && session.city == city
           && session.grade == grade
           && session.sex == sex
        {
            return Ok(ProfileStatus::NothingChanged);
        } else {
            result = ProfileStatus::DataChanged;
        }
    }

    match user_id {
        None => {
            session.firstname = Some(firstname);
            session.lastname = Some(lastname);
            session.grade = grade;
            session.sex = sex;

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
            user.sex = sex;

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

pub fn teacher_infos<T: MedalConnection>(conn: &T, session_token: &str) -> MedalValueResult {
    let session = conn.get_session(&session_token).ensure_logged_in().ok_or(MedalError::NotLoggedIn)?;
    if !session.is_teacher {
        return Err(MedalError::AccessDenied);
    }

    let mut data = json_val::Map::new();
    fill_user_data(&session, &mut data);

    Ok(("teacher".to_string(), data))
}

pub fn admin_index<T: MedalConnection>(conn: &T, session_token: &str) -> MedalValueResult {
    let session = conn.get_session(&session_token)
                      .ensure_logged_in()
                      .ok_or(MedalError::NotLoggedIn)?
                      .ensure_admin()
                      .ok_or(MedalError::AccessDenied)?;

    let mut data = json_val::Map::new();
    fill_user_data(&session, &mut data);

    Ok(("admin".to_string(), data))
}

pub fn admin_search_users<T: MedalConnection>(conn: &T, session_token: &str,
                                              s_data: (Option<i32>,
                                               Option<String>,
                                               Option<String>,
                                               Option<String>,
                                               Option<String>,
                                               Option<String>))
                                              -> MedalValueResult {
    let session = conn.get_session(&session_token)
                      .ensure_logged_in()
                      .ok_or(MedalError::NotLoggedIn)?
                      .ensure_admin()
                      .ok_or(MedalError::AccessDenied)?;

    let mut data = json_val::Map::new();
    fill_user_data(&session, &mut data);

    match conn.get_search_users(s_data) {
        Ok(users) => {
            data.insert("users".to_string(), to_json(&users));
            data.insert("max_results".to_string(), to_json(&200));
            data.insert("num_results".to_string(), to_json(&users.len()));
            data.insert("no_results".to_string(), to_json(&(users.len() == 0)));
            if users.len() > 200 {
                data.insert("more_users".to_string(), to_json(&true));
                data.insert("more_results".to_string(), to_json(&true));
            }
        }
        Err(groups) => {
            data.insert("groups".to_string(), to_json(&groups));
            data.insert("max_results".to_string(), to_json(&200));
            data.insert("num_results".to_string(), to_json(&groups.len()));
            data.insert("no_results".to_string(), to_json(&(groups.len() == 0)));
            if groups.len() > 200 {
                data.insert("more_groups".to_string(), to_json(&true));
                data.insert("more_results".to_string(), to_json(&true));
            }
        }
    };

    Ok(("admin_search_results".to_string(), data))
}

pub fn admin_show_user<T: MedalConnection>(conn: &T, user_id: i32, session_token: &str) -> MedalValueResult {
    let session = conn.get_session(&session_token)
                      .ensure_logged_in()
                      .ok_or(MedalError::NotLoggedIn)?
                      .ensure_teacher_or_admin()
                      .ok_or(MedalError::AccessDenied)?;

    let mut data = json_val::Map::new();

    let (user, opt_group) = conn.get_user_and_group_by_id(user_id).ok_or(MedalError::AccessDenied)?;

    if !session.is_admin() {
        // Check access for teachers
        if let Some(group) = opt_group.clone() {
            if group.admin != session.id {
                return Err(MedalError::AccessDenied);
            }
        } else if user_id != session.id {
            return Err(MedalError::AccessDenied);
        }
    }

    fill_user_data(&session, &mut data);
    fill_user_data_prefix(&user, &mut data, "user_");
    data.insert("user_logincode".to_string(), to_json(&user.logincode));
    data.insert("user_id".to_string(), to_json(&user.id));
    let grade = if user.grade >= 200 {
        "Kein Schüler mehr".to_string()
    } else if user.grade >= 11 {
        format!("{} ({})", user.grade % 100, if user.grade >= 100 { "G9" } else { "G8" })
    } else {
        format!("{}", user.grade)
    };
    data.insert("user_grade".to_string(), to_json(&grade));
    data.insert("user_oauthid".to_string(), to_json(&user.oauth_foreign_id));
    data.insert("user_oauthprovider".to_string(), to_json(&user.oauth_provider));

    if let Some(group) = opt_group {
        data.insert("user_group_id".to_string(), to_json(&group.id));
        data.insert("user_group_name".to_string(), to_json(&group.name));
    }

    let groups: Vec<GroupInfo> =
        conn.get_groups(user_id)
            .iter()
            .map(|g| GroupInfo { id: g.id.unwrap(),
                                 name: g.name.clone(),
                                 tag: g.tag.clone(),
                                 code: g.groupcode.clone() })
            .collect();
    data.insert("user_group".to_string(), to_json(&groups));

    let parts = conn.get_all_participations_complete(user_id);
    let has_protected_participations = parts.iter().any(|p| p.1.protected);

    let pi: Vec<(i32, String)> =
        parts.into_iter()
             .map(|(_, c)| (c.id.unwrap(), format!("{}{}", &c.name, if c.protected { " (*)" } else { "" })))
             .collect();

    data.insert("user_participations".to_string(), to_json(&pi));
    data.insert("has_protected_participations".to_string(), to_json(&has_protected_participations));
    data.insert("can_delete".to_string(),
                to_json(&((!has_protected_participations || session.is_admin()) && groups.len() == 0)));

    Ok(("admin_user".to_string(), data))
}

pub fn admin_delete_user<T: MedalConnection>(conn: &T, user_id: i32, session_token: &str, csrf_token: &str)
                                             -> MedalValueResult {
    let session = conn.get_session(&session_token)
                      .ensure_logged_in()
                      .ok_or(MedalError::NotLoggedIn)?
                      .ensure_teacher_or_admin()
                      .ok_or(MedalError::AccessDenied)?;

    if session.csrf_token != csrf_token {
        return Err(MedalError::CsrfCheckFailed);
    }

    let (_, opt_group) = conn.get_user_and_group_by_id(user_id).ok_or(MedalError::AccessDenied)?;

    if !session.is_admin() {
        // Check access for teachers
        if let Some(group) = opt_group {
            if group.admin != session.id {
                return Err(MedalError::AccessDenied);
            }
        } else {
            return Err(MedalError::AccessDenied);
        }
    }

    let parts = conn.get_all_participations_complete(user_id);
    let has_protected_participations = parts.iter().any(|p| p.1.protected);
    let groups = conn.get_groups(user_id);

    let mut data = json_val::Map::new();
    if has_protected_participations && !session.is_admin() {
        data.insert("reason".to_string(), to_json(&"Benutzer hat Teilnahmen an geschützten Wettbewerben."));
        Ok(("delete_fail".to_string(), data))
    } else if groups.len() > 0 {
        data.insert("reason".to_string(), to_json(&"Benutzer ist Administrator von Gruppen."));
        Ok(("delete_fail".to_string(), data))
    } else {
        conn.delete_user(user_id);
        Ok(("delete_ok".to_string(), data))
    }
}

pub fn admin_move_user_to_group<T: MedalConnection>(conn: &T, user_id: i32, group_id: i32, session_token: &str,
                                                    csrf_token: &str)
                                                    -> MedalValueResult {
    let session = conn.get_session(&session_token)
                      .ensure_logged_in()
                      .ok_or(MedalError::NotLoggedIn)?
                      .ensure_admin()
                      .ok_or(MedalError::AccessDenied)?;

    if session.csrf_token != csrf_token {
        return Err(MedalError::CsrfCheckFailed);
    }

    let (_, opt_group) = conn.get_user_and_group_by_id(user_id).ok_or(MedalError::AccessDenied)?;

    if !session.is_admin() {
        // Check access for teachers
        if let Some(group) = opt_group {
            if group.admin != session.id {
                return Err(MedalError::AccessDenied);
            }
        } else {
            return Err(MedalError::AccessDenied);
        }
    }

    let mut data = json_val::Map::new();
    if conn.get_group_complete(group_id).is_some() {
        if let Some(mut user) = conn.get_user_by_id(user_id) {
            user.managed_by = Some(group_id);
            conn.save_session(user);
            Ok(("delete_ok".to_string(), data))
        } else {
            data.insert("reason".to_string(), to_json(&"Benutzer existiert nicht."));
            Ok(("delete_fail".to_string(), data))
        }
    } else {
        data.insert("reason".to_string(), to_json(&"Gruppe existiert nicht."));
        Ok(("delete_fail".to_string(), data))
    }
}

pub fn admin_show_group<T: MedalConnection>(conn: &T, group_id: i32, session_token: &str) -> MedalValueResult {
    let session = conn.get_session(&session_token)
                      .ensure_logged_in()
                      .ok_or(MedalError::NotLoggedIn)?
                      .ensure_teacher_or_admin()
                      .ok_or(MedalError::AccessDenied)?;

    let group = conn.get_group_complete(group_id).unwrap(); // TODO handle error

    if !session.is_admin() {
        // Check access for teachers
        if group.admin != session.id {
            return Err(MedalError::AccessDenied);
        }
    }

    let mut data = json_val::Map::new();
    fill_user_data(&session, &mut data);

    let gi = GroupInfo { id: group.id.unwrap(),
                         name: group.name.clone(),
                         tag: group.tag.clone(),
                         code: group.groupcode.clone() };

    let v: Vec<MemberInfo> =
        group.members
             .iter()
             .filter(|m| session.is_admin() || m.firstname.is_some() || m.lastname.is_some())
             .map(|m| MemberInfo { id: m.id,
                                   firstname: m.firstname.clone().unwrap_or_else(|| "".to_string()),
                                   lastname: m.lastname.clone().unwrap_or_else(|| "".to_string()),
                                   sex: (match m.sex {
                                            Some(0) | None => "/",
                                            Some(1) => "m",
                                            Some(2) => "w",
                                            Some(3) => "d",
                                            Some(4) => "…",
                                            _ => "?",
                                        }).to_string(),
                                   grade: grade_to_string(m.grade),
                                   logincode: m.logincode.clone().unwrap_or_else(|| "".to_string()) })
             .collect();

    let has_protected_participations = conn.group_has_protected_participations(group_id);

    data.insert("group".to_string(), to_json(&gi));
    data.insert("member".to_string(), to_json(&v));
    data.insert("groupname".to_string(), to_json(&gi.name));
    data.insert("group_admin_id".to_string(), to_json(&group.admin));
    data.insert("has_protected_participations".to_string(), to_json(&has_protected_participations));
    data.insert("can_delete".to_string(), to_json(&(!has_protected_participations || session.is_admin())));

    let user = conn.get_user_by_id(group.admin).ok_or(MedalError::AccessDenied)?;
    data.insert("group_admin_firstname".to_string(), to_json(&user.firstname));
    data.insert("group_admin_lastname".to_string(), to_json(&user.lastname));

    Ok(("admin_group".to_string(), data))
}

pub fn admin_delete_group<T: MedalConnection>(conn: &T, group_id: i32, session_token: &str, csrf_token: &str)
                                              -> MedalValueResult {
    let session = conn.get_session(&session_token)
                      .ensure_logged_in()
                      .ok_or(MedalError::NotLoggedIn)?
                      .ensure_teacher_or_admin()
                      .ok_or(MedalError::AccessDenied)?;

    if session.csrf_token != csrf_token {
        return Err(MedalError::CsrfCheckFailed);
    }

    let group = conn.get_group(group_id).unwrap(); // TODO handle error

    if !session.is_admin() {
        // Check access for teachers
        if group.admin != session.id {
            return Err(MedalError::AccessDenied);
        }
    }

    let mut data = json_val::Map::new();
    if conn.group_has_protected_participations(group_id) && !session.is_admin() {
        data.insert("reason".to_string(), to_json(&"Gruppe hat Mitglieder mit geschützten Teilnahmen."));
        Ok(("delete_fail".to_string(), data))
    } else {
        conn.delete_all_users_for_group(group_id);
        conn.delete_group(group_id);
        Ok(("delete_ok".to_string(), data))
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct SubmissionResult {
    id: i32,
    grade: i32,
    date: String,
}
#[derive(Serialize, Deserialize, Debug)]
struct TaskResult {
    id: i32,
    stars: i32,
    submissions: Vec<SubmissionResult>,
}
#[derive(Serialize, Deserialize, Debug)]
struct TaskgroupResult {
    id: i32,
    name: String,
    tasks: Vec<TaskResult>,
}

pub fn admin_show_participation<T: MedalConnection>(conn: &T, user_id: i32, contest_id: i32, session_token: &str)
                                                    -> MedalValueResult {
    let session = conn.get_session(&session_token)
                      .ensure_logged_in()
                      .ok_or(MedalError::NotLoggedIn)?
                      .ensure_teacher_or_admin()
                      .ok_or(MedalError::AccessDenied)?;

    let (_, opt_group) = conn.get_user_and_group_by_id(user_id).ok_or(MedalError::AccessDenied)?;

    if !session.is_admin() {
        // Check access for teachers
        if let Some(ref group) = opt_group {
            if group.admin != session.id {
                return Err(MedalError::AccessDenied);
            }
        } else {
            return Err(MedalError::AccessDenied);
        }
    }

    let contest = conn.get_contest_by_id_complete(contest_id).ok_or(MedalError::UnknownId)?;

    #[rustfmt::skip]
    let subms: Vec<TaskgroupResult> =
        contest.taskgroups
            .into_iter()
            .map(|tg| TaskgroupResult {
                id: tg.id.unwrap(),
                name: tg.name,
                tasks: tg.tasks
                    .into_iter()
                    .map(|t| TaskResult {
                        id: t.id.unwrap(),
                        stars: t.stars,
                        submissions: conn.get_all_submissions(user_id, t.id.unwrap(), None)
                            .into_iter()
                            .map(|s| SubmissionResult {
                                id: s.id.unwrap(),
                                grade: s.grade,
                                date: self::time::strftime("%e. %b %Y, %H:%M", &self::time::at(s.date)).unwrap(),
                            })
                            .collect(),
                      })
                      .collect(),
               })
               .collect();

    let mut data = json_val::Map::new();

    data.insert("submissions".to_string(), to_json(&subms));
    data.insert("contestid".to_string(), to_json(&contest.id));
    data.insert("contestname".to_string(), to_json(&contest.name));
    data.insert("has_timelimit".to_string(), to_json(&(contest.duration > 0)));

    if let Some(group) = opt_group {
        data.insert("group_id".to_string(), to_json(&group.id));
        data.insert("group_name".to_string(), to_json(&group.name));
    }

    let user = conn.get_user_by_id(user_id).ok_or(MedalError::AccessDenied)?;
    fill_user_data(&session, &mut data);
    fill_user_data_prefix(&user, &mut data, "user_");
    data.insert("user_id".to_string(), to_json(&user.id));

    let participation = conn.get_participation(user.id, contest_id).ok_or(MedalError::AccessDenied)?;
    data.insert("start_date".to_string(),
                to_json(&self::time::strftime("%e. %b %Y, %H:%M", &self::time::at(participation.start)).unwrap()));

    data.insert("can_delete".to_string(), to_json(&(!contest.protected || session.is_admin.unwrap_or(false))));
    Ok(("admin_participation".to_string(), data))
}

pub fn admin_delete_participation<T: MedalConnection>(conn: &T, user_id: i32, contest_id: i32, session_token: &str,
                                                      csrf_token: &str)
                                                      -> MedalValueResult {
    let session = conn.get_session(&session_token)
                      .ensure_logged_in()
                      .ok_or(MedalError::NotLoggedIn)?
                      .ensure_teacher_or_admin()
                      .ok_or(MedalError::AccessDenied)?;

    if session.csrf_token != csrf_token {
        return Err(MedalError::CsrfCheckFailed);
    }

    let (user, opt_group) = conn.get_user_and_group_by_id(user_id).ok_or(MedalError::AccessDenied)?;
    let _part = conn.get_participation(user.id, contest_id).ok_or(MedalError::AccessDenied)?;
    let contest = conn.get_contest_by_id_complete(contest_id).ok_or(MedalError::UnknownId)?;

    if !session.is_admin() {
        // Check access for teachers
        if contest.protected {
            return Err(MedalError::AccessDenied);
        }

        if let Some(group) = opt_group {
            if group.admin != session.id {
                return Err(MedalError::AccessDenied);
            }
        } else {
            return Err(MedalError::AccessDenied);
        }
    }

    let mut data = json_val::Map::new();
    fill_user_data(&session, &mut data);

    conn.delete_participation(user_id, contest_id);
    Ok(("delete_ok".to_string(), data))
}

pub fn admin_show_contests<T: MedalConnection>(conn: &T, session_token: &str) -> MedalValueResult {
    let session = conn.get_session(&session_token)
                      .ensure_logged_in()
                      .ok_or(MedalError::NotLoggedIn)?
                      .ensure_admin()
                      .ok_or(MedalError::AccessDenied)?;

    let mut data = json_val::Map::new();
    fill_user_data(&session, &mut data);

    let mut contests: Vec<_> = conn.get_contest_list().into_iter().map(|contest| (contest.id, contest.name)).collect();
    contests.sort(); // Sort by id (sorts by natural order on fields)
    contests.reverse();

    data.insert("contests".to_string(), to_json(&contests));

    Ok(("admin_contests".to_string(), data))
}

pub fn admin_contest_export<T: MedalConnection>(conn: &T, contest_id: i32, session_token: &str) -> MedalResult<String> {
    conn.get_session(&session_token)
        .ensure_logged_in()
        .ok_or(MedalError::NotLoggedIn)?
        .ensure_admin()
        .ok_or(MedalError::AccessDenied)?;

    let contest = conn.get_contest_by_id_complete(contest_id).ok_or(MedalError::UnknownId)?;

    let taskgroup_ids: Vec<(i32, String)> =
        contest.taskgroups.into_iter().map(|tg| (tg.id.unwrap(), tg.name)).collect();
    let filename = format!("contest_{}__{}__{}.csv",
                           contest_id,
                           self::time::strftime("%F_%H-%M-%S", &self::time::now()).unwrap(),
                           helpers::make_filename_secret());

    conn.export_contest_results_to_file(contest_id, &taskgroup_ids, &format!("./export/{}", filename));

    Ok(filename)
}

pub fn admin_show_cleanup<T: MedalConnection>(conn: &T, session_token: &str) -> MedalValueResult {
    let session = conn.get_session(&session_token)
                      .ensure_logged_in()
                      .ok_or(MedalError::NotLoggedIn)?
                      .ensure_admin()
                      .ok_or(MedalError::AccessDenied)?;

    let mut data = json_val::Map::new();
    fill_user_data(&session, &mut data);

    Ok(("admin_cleanup".to_string(), data))
}

pub fn admin_do_cleanup<T: MedalConnection>(conn: &T, session_token: &str, csrf_token: &str) -> MedalValueResult {
    let session = conn.get_session(&session_token)
                      .ensure_logged_in()
                      .ok_or(MedalError::NotLoggedIn)?
                      .ensure_admin()
                      .ok_or(MedalError::AccessDenied)?;

    if session.csrf_token != csrf_token {
        return Err(MedalError::CsrfCheckFailed);
    }

    let now = time::get_time();
    let maxstudentage = now - time::Duration::days(180); // Delete managed users after 180 days of inactivity
    let maxteacherage = now - time::Duration::days(1095); // Delete teachers after 3 years of inactivity
    let maxage = now - time::Duration::days(3650); // Delete every user after 10 years of inactivity

    let result = conn.remove_old_users_and_groups(maxstudentage, Some(maxteacherage), Some(maxage));

    let mut data = json_val::Map::new();
    if let Ok((n_user, n_group, n_teacher, n_other)) = result {
        let infodata = format!(",\"n_user\":{},\"n_group\":{},\"n_teacher\":{},\"n_other\":{}",
                               n_user, n_group, n_teacher, n_other);
        data.insert("data".to_string(), to_json(&infodata));
        Ok(("delete_ok".to_string(), data))
    } else {
        data.insert("reason".to_string(), to_json(&"Fehler."));
        Ok(("delete_fail".to_string(), data))
    }
}

pub fn do_session_cleanup<T: MedalConnection>(conn: &T) -> MedalValueResult {
    let now = time::get_time();
    let maxage = now - time::Duration::days(30); // Delete all temporary sessions after 30 days

    let result = conn.remove_temporary_sessions(maxage);

    let mut data = json_val::Map::new();
    if let Ok((n_session, last_cleanup)) = result {
        let infodata = format!(",\"n_session\":{},\"last_cleanup\":{:?}", n_session, last_cleanup);
        data.insert("data".to_string(), to_json(&infodata));
        Ok(("delete_ok".to_string(), data))
    } else {
        data.insert("reason".to_string(), to_json(&"Fehler."));
        Ok(("delete_fail".to_string(), data))
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum UserType {
    User,
    Teacher,
    Admin,
}

pub enum UserSex {
    Female,
    Male,
    Unknown,
}

pub struct ForeignUserData {
    pub foreign_id: String,
    pub foreign_type: UserType,
    pub sex: UserSex,
    pub firstname: String,
    pub lastname: String,
}

pub fn login_oauth<T: MedalConnection>(conn: &T, user_data: ForeignUserData, oauth_provider_id: String)
                                       -> Result<(String, bool), (String, json_val::Map<String, json_val::Value>)> {
    match conn.login_foreign(None,
                             &oauth_provider_id,
                             &user_data.foreign_id,
                             (user_data.foreign_type != UserType::User,
                              user_data.foreign_type == UserType::Admin,
                              &user_data.firstname,
                              &user_data.lastname,
                              match user_data.sex {
                                  UserSex::Male => Some(1),
                                  UserSex::Female => Some(2),
                                  UserSex::Unknown => Some(0),
                              })) {
        Ok((session_token, last_activity)) => {
            let redirect_profile = if let Some(last_activity) = last_activity {
                let now = time::get_time();
                now - last_activity > time::Duration::days(60)
            } else {
                true
            };
            Ok((session_token, redirect_profile))
        }
        Err(()) => {
            let mut data = json_val::Map::new();
            data.insert("reason".to_string(), to_json(&"OAuth-Login failed.".to_string()));
            Err(("login".to_owned(), data))
        }
    }
}
