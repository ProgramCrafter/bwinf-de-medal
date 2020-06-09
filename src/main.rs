/*  medal                                                                                                            *\
 *  Copyright (C) 2020  Bundesweite Informatikwettbewerbe                                                            *
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

#![cfg_attr(feature = "strict", deny(warnings))]

#[macro_use]
extern crate iron;
#[macro_use]
extern crate router;
#[macro_use]
extern crate serde_derive;

extern crate handlebars_iron;
extern crate iron_sessionstorage;
extern crate mount;
extern crate params;
extern crate persistent;
extern crate rand;
extern crate reqwest;
extern crate serde_json;
extern crate serde_yaml;
extern crate staticfile;
extern crate structopt;
extern crate time;
extern crate urlencoded;

#[cfg(feature = "postgres")]
extern crate postgres;
#[cfg(feature = "rusqlite")]
extern crate rusqlite;
#[cfg(feature = "webbrowser")]
extern crate webbrowser;

pub mod config;
pub mod contestreader_yaml;
pub mod core;
pub mod db_conn;
pub mod helpers;
pub mod oauth_provider;

mod db_apply_migrations;
mod db_conn_postgres;
mod db_conn_sqlite_new;
mod db_objects;
mod webfw_iron;

use db_conn::{MedalConnection, MedalObject};
use db_objects::*;
use helpers::SetPassword;
use webfw_iron::start_server;

use config::Config;
use structopt::StructOpt;

use std::path::{Path, PathBuf};

fn read_contest(p: &PathBuf) -> Option<Contest> {
    use std::fs::File;
    use std::io::Read;

    let mut file = File::open(p).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();

    contestreader_yaml::parse_yaml(&contents,
                                   p.file_name().to_owned()?.to_str()?,
                                   &format!("{}/", p.parent().unwrap().to_str()?))
}

fn get_all_contest_info(task_dir: &str) -> Vec<Contest> {
    fn walk_me_recursively(p: &PathBuf, contests: &mut Vec<Contest>) {
        if let Ok(paths) = std::fs::read_dir(p) {
            for path in paths {
                let p = path.unwrap().path();
                walk_me_recursively(&p, contests);
            }
        }

        if p.file_name().unwrap().to_string_lossy().to_string().ends_with(".yaml") {
            read_contest(p).map(|contest| contests.push(contest));
        };
    };

    let mut contests = Vec::new();
    match std::fs::read_dir(task_dir) {
        Err(why) => println!("Error opening tasks directory! {:?}", why.kind()),
        Ok(paths) => {
            for path in paths {
                walk_me_recursively(&path.unwrap().path(), &mut contests);
            }
        }
    };

    contests
}

fn refresh_all_contests<C>(conn: &mut C)
    where C: MedalConnection,
          db_objects::Contest: db_conn::MedalObject<C>
{
    conn.reset_all_contest_visibilities();
    conn.reset_all_taskgroup_visibilities();

    let v = get_all_contest_info("tasks/");

    for mut contest_info in v {
        contest_info.save(conn);
    }
}

fn add_admin_user<C>(conn: &mut C, resetpw: bool)
    where C: MedalConnection {
    let mut admin = match conn.get_user_by_id(1) {
        None => {
            print!("New Database. Creating new admin user with credentials 'admin':");
            conn.new_session("")
        }
        Some(user) => {
            if !resetpw {
                return;
            }
            print!("Request to reset admin password. Set credentials 'admin':");
            user
        }
    };

    use rand::{distributions::Alphanumeric, thread_rng, Rng};

    let password: String = thread_rng().sample_iter(&Alphanumeric)
                                       .filter(|x| {
                                           let x = *x;
                                           !(x == 'l' || x == 'I' || x == '1' || x == 'O' || x == 'o' || x == '0')
                                       })
                                       .take(8)
                                       .collect();
    print!("'{}', ", &password);

    let logincode: String = thread_rng().sample_iter(&Alphanumeric)
                                       .filter(|x| {
                                           let x = *x;
                                           !(x == 'l' || x == 'I' || x == '1' || x == 'O' || x == 'o' || x == '0')
                                       })
                                       .take(8)
                                       .collect();
    let logincode = format!("a{}", logincode);
    print!(" logincode:'{}' …", &logincode);

    admin.username = Some("admin".into());
    admin.logincode = Some(logincode);
    match admin.set_password(&password) {
        None => println!(" FAILED! (Password hashing error)"),
        _ => {
            conn.save_session(admin);
            println!(" Done");
        }
    }
}

fn prepare_and_start_server<C>(mut conn: C, config: Config, onlycontestscan: bool, resetadminpw: bool)
    where C: MedalConnection + std::marker::Send + 'static,
          db_objects::Contest: db_conn::MedalObject<C>
{
    db_apply_migrations::test(&mut conn);

    if onlycontestscan || config.no_contest_scan == Some(false) {
        print!("Scanning for contests …");
        refresh_all_contests(&mut conn);
        println!(" Done")
    }

    if !onlycontestscan {
        add_admin_user(&mut conn, resetadminpw);

        #[cfg(feature = "webbrowser")]
        let self_url = config.self_url.clone();
        #[cfg(feature = "webbrowser")]
        let open_browser = config.open_browser;

        match start_server(conn, config) {
            Ok(_) => {
                println!("Server started");

                #[cfg(feature = "webbrowser")]
                {
                    if let (Some(self_url), Some(true)) = (self_url, open_browser) {
                        open_browser_window(&self_url);
                    }
                }
            }
            Err(_) => println!("Error on server start …"),
        };

        println!("Could not run server. Is the port already in use?");
    }
}

#[cfg(feature = "webbrowser")]
fn open_browser_window(self_url: &str) {
    match webbrowser::open(&self_url) {
        Ok(_) => (),
        Err(e) => println!("Error while opening webbrowser: {:?}", e),
    }
}

fn main() {
    let opt = config::Opt::from_args();

    #[cfg(feature = "debug")]
    println!("Options: {:#?}", opt);

    let mut config = config::read_config_from_file(&opt.configfile);

    #[cfg(feature = "debug")]
    println!("Config: {:#?}", config);

    // Let options override config values
    opt.databasefile.map(|x| config.database_file = Some(x));
    opt.databaseurl.map(|x| config.database_url = Some(x));
    opt.teacherpage.map(|x| config.teacher_page = Some(x));
    opt.port.map(|x| config.port = Some(x));
    config.no_contest_scan = if opt.nocontestscan { Some(true) } else { config.no_contest_scan };
    config.open_browser = if opt.openbrowser { Some(true) } else { config.open_browser };
    config.disable_results_page = if opt.disableresultspage { Some(true) } else { config.disable_results_page };

    // Use default database file if none set
    config.database_file.get_or_insert(Path::new("medal.db").to_owned());

    #[cfg(feature = "debug")]
    println!("Using config: {:#?}", config);

    #[cfg(feature = "postgres")]
    {
        if let Some(url) = config.database_url.clone() {
            print!("Using database {} … ", &url);
            let conn = postgres::Connection::connect(url, postgres::TlsMode::None).unwrap();
            println!("Connected");

            prepare_and_start_server(conn, config, opt.onlycontestscan, opt.resetadminpw);
            return;
        }
    }

    #[cfg(feature = "rusqlite")]
    {
        if let Some(path) = config.database_file.clone() {
            print!("Using database file {} … ", &path.to_str().unwrap_or("<unprintable filename>"));
            let conn = rusqlite::Connection::open(path).unwrap();
            println!("Connected");

            prepare_and_start_server(conn, config, opt.onlycontestscan, opt.resetadminpw);
            return;
        }
    }

    println!("No database configured. Try enableing the 'rusqlite' feature during compilation.\nLeaving now.");
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::StatusCode;

    fn start_server_and_fn<F>(port: u16, set_user: Option<(String, String, bool)>, f: F)
        where F: Fn() {
        use std::sync::mpsc::channel;
        use std::{thread, time};
        let (start_tx, start_rx) = channel();
        let (stop_tx, stop_rx) = channel();

        thread::spawn(move || {
            let mut conn = rusqlite::Connection::open_in_memory().unwrap();
            db_apply_migrations::test(&mut conn);

            if let Some(user) = set_user {
                let mut test_user = conn.new_session("");
                test_user.username = Some(user.0);
                test_user.is_teacher = user.2;
                test_user.set_password(&user.1).expect("Set Password did not work correctly.");
                conn.save_session(test_user);
            }

            // ID: 1, gets renamed
            let mut contest = Contest::new("directory".to_string(),
                                           "public.yaml".to_string(),
                                           "RenamedContestName".to_string(),
                                           1,
                                           true,
                                           None,
                                           None,
                                           None,
                                           None,
                                           None,
                                           None,
                                           None);
            contest.save(&conn);

            // ID: 1
            let mut contest = Contest::new("directory".to_string(),
                                           "public.yaml".to_string(),
                                           "PublicContestName".to_string(),
                                           1,
                                           true,
                                           None,
                                           None,
                                           None,
                                           None,
                                           None,
                                           None,
                                           None);
            let mut taskgroup = Taskgroup::new("TaskgroupName".to_string(), None);
            let task = Task::new("taskdir1".to_string(), 3); // ID: 1
            taskgroup.tasks.push(task);
            let task = Task::new("taskdir2".to_string(), 4); // ID: 2
            taskgroup.tasks.push(task);
            contest.taskgroups.push(taskgroup);
            contest.save(&conn);

            // ID: 2
            let mut contest = Contest::new("directory".to_string(),
                                           "private.yaml".to_string(),
                                           "PrivateContestName".to_string(),
                                           1,
                                           false,
                                           None,
                                           None,
                                           None,
                                           None,
                                           None,
                                           None,
                                           None);
            let mut taskgroup = Taskgroup::new("TaskgroupName".to_string(), None);
            let task = Task::new("taskdir1".to_string(), 3); // ID: 3
            taskgroup.tasks.push(task);
            let task = Task::new("taskdir2".to_string(), 4); // ID: 4
            taskgroup.tasks.push(task);
            contest.taskgroups.push(taskgroup);
            contest.save(&conn);

            // ID: 3
            let mut contest = Contest::new("directory".to_string(),
                                           "infinte.yaml".to_string(),
                                           "InfiniteContestName".to_string(),
                                           0,
                                           true,
                                           None,
                                           None,
                                           None,
                                           None,
                                           None,
                                           None,
                                           None);
            let mut taskgroup = Taskgroup::new("TaskgroupRenameName".to_string(), None);
            let task = Task::new("taskdir1".to_string(), 3); // ID: 5
            taskgroup.tasks.push(task);
            let task = Task::new("taskdir2".to_string(), 4); // ID: 6
            taskgroup.tasks.push(task);
            contest.taskgroups.push(taskgroup);
            contest.save(&conn);

            let mut taskgroup = Taskgroup::new("TaskgroupNewName".to_string(), None);
            let task = Task::new("taskdir1".to_string(), 3); // ID: 5
            taskgroup.tasks.push(task);
            let task = Task::new("taskdir2".to_string(), 4); // ID: 6
            taskgroup.tasks.push(task);
            contest.taskgroups.push(taskgroup);
            contest.save(&conn);

            let mut config = config::read_config_from_file(Path::new("thisfileshoudnotexist"));
            config.port = Some(port);
            config.cookie_signing_secret = Some("testtesttesttesttesttesttesttest".to_string());
            let mut srvr = start_server(conn, config).expect(&format!("Could not start server on port {}", port));

            // Message server started
            start_tx.send(()).unwrap();

            // Wait for test to finish
            stop_rx.recv().unwrap();

            srvr.close().unwrap();
        });

        // Wait for server to start
        start_rx.recv().unwrap();
        thread::sleep(time::Duration::from_millis(100));

        // Run test code
        f();

        // Message test finished
        stop_tx.send(()).unwrap();
    }

    fn login(port: u16, client: &reqwest::Client, username: &str, password: &str) -> reqwest::Response {
        let params = [("username", username), ("password", password)];
        let resp = client.post(&format!("http://localhost:{}/login", port)).form(&params).send().unwrap();
        resp
    }

    fn login_code(port: u16, client: &reqwest::Client, code: &str) -> reqwest::Response {
        let params = [("code", code)];
        let resp = client.post(&format!("http://localhost:{}/clogin", port)).form(&params).send().unwrap();
        resp
    }

    #[test]
    fn start_server_and_check_requests() {
        start_server_and_fn(8080, None, || {
            let mut resp = reqwest::get("http://localhost:8080").unwrap();
            assert_eq!(resp.status(), StatusCode::OK);

            let content = resp.text().unwrap();
            assert!(content.contains("Jugendwettbewerb Informatik</h1>"));
            assert!(!content.contains("Error"));
            assert!(!content.contains("Gruppenverwaltung"));

            let mut resp = reqwest::get("http://localhost:8080/contest").unwrap();
            assert_eq!(resp.status(), StatusCode::OK);

            let content = resp.text().unwrap();
            assert!(content.contains("<h1>Wettbewerbe</h1>"));
            assert!(!content.contains("Error"));

            let mut resp = reqwest::get("http://localhost:8080/group").unwrap();
            let content = resp.text().unwrap();
            assert!(content.contains("<h1>Login</h1>"));
        })
    }

    #[test]
    fn check_login_wrong_credentials() {
        start_server_and_fn(8081, None, || {
            let client = reqwest::Client::new();

            let mut resp = login(8081, &client, "nonexistingusername", "wrongpassword");
            assert_eq!(resp.status(), StatusCode::OK);

            let content = resp.text().unwrap();
            assert!(content.contains("<h1>Login</h1>"));
            assert!(content.contains("Login fehlgeschlagen."));
            assert!(!content.contains("Error"));

            let mut resp = login_code(8081, &client, "g23AgaV");
            assert_eq!(resp.status(), StatusCode::OK);

            let content = resp.text().unwrap();
            assert!(content.contains("<h1>Login</h1>"));
            assert!(content.contains("Kein gültiger Code."));
            assert!(!content.contains("Error"));

            let mut resp = login_code(8081, &client, "u9XuAbH7p");
            assert_eq!(resp.status(), StatusCode::OK);

            let content = resp.text().unwrap();
            assert!(content.contains("<h1>Login</h1>"));
            assert!(content.contains("Kein gültiger Code."));
            assert!(!content.contains("Error"));
        })
    }

    #[test]
    fn check_login() {
        start_server_and_fn(8082, Some(("testusr".to_string(), "testpw".to_string(), false)), || {
            let client = reqwest::Client::builder().cookie_store(true)
                                                   .redirect(reqwest::RedirectPolicy::none())
                                                   .build()
                                                   .unwrap();

            let mut resp = login(8082, &client, "testusr", "testpw");
            assert_eq!(resp.status(), StatusCode::FOUND);

            let content = resp.text().unwrap();
            assert!(!content.contains("Error"));

            let mut set_cookie = resp.headers().get_all("Set-Cookie").iter();
            assert!(set_cookie.next().is_some());
            assert!(set_cookie.next().is_none());

            let location = resp.headers().get(reqwest::header::LOCATION).unwrap().to_str().unwrap();
            assert_eq!(location, "http://localhost:8082/");

            let mut resp = client.get(location).send().unwrap();
            assert_eq!(resp.status(), StatusCode::OK);

            let content = resp.text().unwrap();
            assert!(!content.contains("Error"));
            assert!(!content.contains("Gruppenverwaltung"));
            assert!(content.contains("Eingeloggt als <em>testusr</em>"));
            assert!(content.contains("Jugendwettbewerb Informatik</h1>"));
        })
    }

    #[test]
    fn check_logout() {
        start_server_and_fn(8083, Some(("testusr".to_string(), "testpw".to_string(), false)), || {
            let client = reqwest::Client::builder().cookie_store(true)
                                                   .redirect(reqwest::RedirectPolicy::none())
                                                   .build()
                                                   .unwrap();

            let resp = login(8083, &client, "testusr", "testpw");
            assert_eq!(resp.status(), StatusCode::FOUND);

            let resp = client.get("http://localhost:8083/logout").send().unwrap();
            assert_eq!(resp.status(), StatusCode::FOUND);

            let mut resp = client.get("http://localhost:8083").send().unwrap();
            assert_eq!(resp.status(), StatusCode::OK);

            let content = resp.text().unwrap();
            assert!(content.contains("Benutzername"));
            assert!(content.contains("Passwort"));
            assert!(content.contains("Gruppencode / Teilnahmecode"));
            assert!(content.contains("Jugendwettbewerb Informatik</h1>"));
        })
    }

    #[test]
    fn check_group_creation_and_group_code_login() {
        start_server_and_fn(8084, Some(("testusr".to_string(), "testpw".to_string(), true)), || {
            let client = reqwest::Client::builder().cookie_store(true)
                                                   .redirect(reqwest::RedirectPolicy::none())
                                                   .build()
                                                   .unwrap();

            let resp = login(8084, &client, "testusr", "testpw");
            assert_eq!(resp.status(), StatusCode::FOUND);

            let mut resp = client.get("http://localhost:8084").send().unwrap();
            assert_eq!(resp.status(), StatusCode::OK);

            let content = resp.text().unwrap();
            assert!(content.contains("[Lehrer]"));
            assert!(content.contains("Gruppenverwaltung"));

            let mut resp = client.get("http://localhost:8084/group/").send().unwrap();
            assert_eq!(resp.status(), StatusCode::OK);

            let content = resp.text().unwrap();
            assert!(content.contains("Gruppe anlegen"));

            let params = [("name", "WrongGroupname"), ("tag", "WrongMarker"), ("csrf_token", "76CfTPJaoz")];
            let resp = client.post("http://localhost:8084/group/").form(&params).send().unwrap();
            assert_eq!(resp.status(), StatusCode::FORBIDDEN);

            let pos = content.find("type=\"hidden\" name=\"csrf_token\" value=\"").expect("CSRF-Token not found");
            let csrf = &content[pos + 39..pos + 49];
            let params = [("name", "Groupname"), ("tag", "Marker"), ("csrf_token", csrf)];
            let resp = client.post("http://localhost:8084/group/").form(&params).send().unwrap();
            assert_eq!(resp.status(), StatusCode::FOUND);

            let mut resp = client.get("http://localhost:8084/group/").send().unwrap();
            let content = resp.text().unwrap();
            assert!(!content.contains("WrongGroupname"));

            let pos = content.find("<td><a href=\"/group/1\">Groupname</a></td>").expect("Group not found");
            let groupcode = &content[pos + 58..pos + 65];

            // New client to test group code login
            let client = reqwest::Client::builder().cookie_store(true)
                                                   .redirect(reqwest::RedirectPolicy::none())
                                                   .build()
                                                   .unwrap();

            let resp = login_code(8084, &client, groupcode);
            assert_eq!(resp.status(), StatusCode::FOUND);

            let mut set_cookie = resp.headers().get_all("Set-Cookie").iter();
            assert!(set_cookie.next().is_some());
            assert!(set_cookie.next().is_none());

            let location = resp.headers().get(reqwest::header::LOCATION).unwrap().to_str().unwrap();
            assert_eq!(location, "http://localhost:8084/profile?status=firstlogin");

            let mut resp = client.get(location).send().unwrap();
            let content = resp.text().unwrap();

            let pos = content.find("<p>Login-Code: ").expect("Logincode not found");
            let logincode = &content[pos + 15..pos + 24];

            // New client to test login code login
            let client = reqwest::Client::builder().cookie_store(true)
                                                   .redirect(reqwest::RedirectPolicy::none())
                                                   .build()
                                                   .unwrap();

            let resp = login_code(8084, &client, logincode);
            assert_eq!(resp.status(), StatusCode::FOUND);

            let location = resp.headers().get(reqwest::header::LOCATION).unwrap().to_str().unwrap();
            assert_eq!(location, "http://localhost:8084/");

            let mut resp = client.get(location).send().unwrap();
            let content = resp.text().unwrap();
            assert!(content.contains("Eingeloggt als <em></em>"));
        })
    }

    #[test]
    fn check_contest_start() {
        start_server_and_fn(8085, Some(("testusr".to_string(), "testpw".to_string(), false)), || {
            let client = reqwest::Client::builder().cookie_store(true)
                                                   .redirect(reqwest::RedirectPolicy::none())
                                                   .build()
                                                   .unwrap();

            let resp = login(8085, &client, "testusr", "testpw");
            assert_eq!(resp.status(), StatusCode::FOUND);

            let mut resp = client.get("http://localhost:8085/contest/").send().unwrap();
            assert_eq!(resp.status(), StatusCode::OK);

            let content = resp.text().unwrap();
            assert!(content.contains("PublicContestName"));
            assert!(content.contains("InfiniteContestName"));
            //assert!(content.contains("PrivateContestName"));
            assert!(!content.contains("WrongContestName"));
            assert!(!content.contains("RenamedContestName"));
            assert!(content.contains("<a href=\"/contest/1\">PublicContestName</a>"));

            let mut resp = client.get("http://localhost:8085/contest/1").send().unwrap();
            assert_eq!(resp.status(), StatusCode::OK);

            let content = resp.text().unwrap();
            assert!(content.contains("PublicContestName"));
            assert!(!content.contains("InfiniteContestName"));
            assert!(!content.contains("PrivateContestName"));
            assert!(!content.contains("WrongContestName"));
            assert!(!content.contains("RenamedContestName"));

            let params = [("csrf_token", "76CfTPJaoz")];
            let resp = client.post("http://localhost:8085/contest/1").form(&params).send().unwrap();
            assert_eq!(resp.status(), StatusCode::FORBIDDEN);

            let pos = content.find("type=\"hidden\" name=\"csrf_token\" value=\"").expect("CSRF-Token not found");
            let csrf = &content[pos + 39..pos + 49];
            let params = [("csrf_token", csrf)];
            let resp = client.post("http://localhost:8085/contest/1").form(&params).send().unwrap();
            assert_eq!(resp.status(), StatusCode::FOUND);

            let mut resp = client.get("http://localhost:8085/contest/1").send().unwrap();
            assert_eq!(resp.status(), StatusCode::OK);

            let content = resp.text().unwrap();
            assert!(content.contains("<a href=\"/task/1\">☆☆☆</a></li>"));
            assert!(content.contains("<a href=\"/task/2\">☆☆☆☆</a></li>"));
        })
    }

    #[test]
    fn check_task_load_save() {
        start_server_and_fn(8086, None, || {
            let client = reqwest::Client::builder().cookie_store(true)
                                                   .redirect(reqwest::RedirectPolicy::none())
                                                   .build()
                                                   .unwrap();

            let resp = client.get("http://localhost:8086/contest/3").send().unwrap();
            assert_eq!(resp.status(), StatusCode::OK);

            let mut resp = client.get("http://localhost:8086/task/5").send().unwrap();
            assert_eq!(resp.status(), StatusCode::OK);

            let content = resp.text().unwrap();
            let pos = content.find("#taskid=5&csrftoken=").expect("CSRF-Token not found");
            let csrf = &content[pos + 20..pos + 30];

            let mut resp = client.get("http://localhost:8086/load/5").send().unwrap();
            assert_eq!(resp.status(), StatusCode::OK);

            let content = resp.text().unwrap();
            assert_eq!(content, "{}");

            let params = [("data", "WrongData"), ("grade", "1"), ("csrf_token", "FNQU4QsEMY")];
            let resp = client.post("http://localhost:8086/save/5").form(&params).send().unwrap();
            assert_eq!(resp.status(), StatusCode::FORBIDDEN);

            // Check that the illegitimate request did not actually change anything
            let mut resp = client.get("http://localhost:8086/load/5").send().unwrap();
            assert_eq!(resp.status(), StatusCode::OK);

            let content = resp.text().unwrap();
            assert_eq!(content, "{}");

            let mut resp = client.get("http://localhost:8086/contest/3").send().unwrap();
            assert_eq!(resp.status(), StatusCode::OK);

            let content = resp.text().unwrap();
            assert!(content.contains("<a href=\"/task/5\">☆☆☆</a></li>"));
            assert!(content.contains("<a href=\"/task/6\">☆☆☆☆</a></li>"));

            let params = [("data", "SomeData"), ("grade", "2"), ("csrf_token", csrf)];
            let mut resp = client.post("http://localhost:8086/save/5").form(&params).send().unwrap();
            assert_eq!(resp.status(), StatusCode::OK);

            let content = resp.text().unwrap();
            assert_eq!(content, "{}");

            let mut resp = client.get("http://localhost:8086/load/5").send().unwrap();
            assert_eq!(resp.status(), StatusCode::OK);

            let content = resp.text().unwrap();
            assert_eq!(content, "SomeData");

            let mut resp = client.get("http://localhost:8086/contest/3").send().unwrap();
            assert_eq!(resp.status(), StatusCode::OK);

            let content = resp.text().unwrap();
            assert!(content.contains("<a href=\"/task/5\">★★☆</a></li>"));
            assert!(content.contains("<a href=\"/task/6\">☆☆☆☆</a></li>"));
        })
    }

    #[test]
    fn check_task_load_save_logged_in() {
        start_server_and_fn(8087, Some(("testusr".to_string(), "testpw".to_string(), false)), || {
            let client = reqwest::Client::builder().cookie_store(true)
                                                   .redirect(reqwest::RedirectPolicy::none())
                                                   .build()
                                                   .unwrap();

            let resp = login(8087, &client, "testusr", "testpw");
            assert_eq!(resp.status(), StatusCode::FOUND);

            let mut resp = client.get("http://localhost:8087/contest/1").send().unwrap();
            assert_eq!(resp.status(), StatusCode::OK);

            let content = resp.text().unwrap();
            let pos = content.find("type=\"hidden\" name=\"csrf_token\" value=\"").expect("CSRF-Token not found");
            let csrf = &content[pos + 39..pos + 49];
            let params = [("csrf_token", csrf)];
            let resp = client.post("http://localhost:8087/contest/1").form(&params).send().unwrap();
            assert_eq!(resp.status(), StatusCode::FOUND);

            let mut resp = client.get("http://localhost:8087/task/1").send().unwrap();
            assert_eq!(resp.status(), StatusCode::OK);

            let content = resp.text().unwrap();
            let pos = content.find("#taskid=1&csrftoken=").expect("CSRF-Token not found");
            let csrf = &content[pos + 20..pos + 30];

            let mut resp = client.get("http://localhost:8087/load/1").send().unwrap();
            assert_eq!(resp.status(), StatusCode::OK);

            let content = resp.text().unwrap();
            assert_eq!(content, "{}");

            let params = [("data", "WrongData"), ("grade", "1"), ("csrf_token", "FNQU4QsEMY")];
            let resp = client.post("http://localhost:8087/save/1").form(&params).send().unwrap();
            assert_eq!(resp.status(), StatusCode::FORBIDDEN);

            // Check that the illigal request did not actually change anything
            let mut resp = client.get("http://localhost:8087/load/1").send().unwrap();
            assert_eq!(resp.status(), StatusCode::OK);

            let content = resp.text().unwrap();
            assert_eq!(content, "{}");

            let mut resp = client.get("http://localhost:8087/contest/1").send().unwrap();
            assert_eq!(resp.status(), StatusCode::OK);

            let content = resp.text().unwrap();
            assert!(content.contains("<a href=\"/task/1\">☆☆☆</a></li>"));
            assert!(content.contains("<a href=\"/task/2\">☆☆☆☆</a></li>"));

            let params = [("data", "SomeData"), ("grade", "2"), ("csrf_token", csrf)];
            let mut resp = client.post("http://localhost:8087/save/1").form(&params).send().unwrap();
            assert_eq!(resp.status(), StatusCode::OK);

            let content = resp.text().unwrap();
            assert_eq!(content, "{}");

            let mut resp = client.get("http://localhost:8087/load/1").send().unwrap();
            assert_eq!(resp.status(), StatusCode::OK);

            let content = resp.text().unwrap();
            assert_eq!(content, "SomeData");

            let mut resp = client.get("http://localhost:8087/contest/1").send().unwrap();
            assert_eq!(resp.status(), StatusCode::OK);

            let content = resp.text().unwrap();
            assert!(content.contains("<a href=\"/task/1\">★★☆</a></li>"));
            assert!(content.contains("<a href=\"/task/2\">☆☆☆☆</a></li>"));
        })
    }

    #[test]
    fn check_taskgroup_rename() {
        start_server_and_fn(8088, None, || {
            let client = reqwest::Client::builder().cookie_store(true)
                                                   .redirect(reqwest::RedirectPolicy::none())
                                                   .build()
                                                   .unwrap();

            let mut resp = client.get("http://localhost:8088/contest/3").send().unwrap();
            assert_eq!(resp.status(), StatusCode::OK);

            let content = resp.text().unwrap();
            println!("{}", content);
            assert!(content.contains("TaskgroupNewName"));
            assert!(!content.contains("TaskgroupRenameName"));

            let mut resp = client.get("http://localhost:8088/task/5").send().unwrap();
            assert_eq!(resp.status(), StatusCode::OK);

            let content = resp.text().unwrap();
            assert!(content.contains("TaskgroupNewName"));
            assert!(!content.contains("TaskgroupRenameName"));
        })
    }
}
