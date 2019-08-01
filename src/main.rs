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
#[cfg(feature = "postgres")]
extern crate postgres;
extern crate rand;
extern crate reqwest;
#[cfg(feature = "rusqlite")]
extern crate rusqlite;
extern crate serde_json;
extern crate serde_yaml;
extern crate staticfile;
extern crate structopt;
extern crate time;
extern crate urlencoded;
#[cfg(feature = "webbrowser")]
extern crate webbrowser;

mod db_apply_migrations;
pub mod db_conn;
mod db_conn_postgres;
mod db_conn_sqlite;
mod db_objects;
pub mod config;
pub mod contestreader_yaml;
pub mod functions;
pub mod oauth_provider;
mod webfw_iron;

use db_conn::{MedalConnection, MedalObject};
use functions::SetPassword; // TODO: Refactor, so we don't need to take this from there!

use db_objects::*;

use webfw_iron::start_server;

use std::fs;
use std::path;

use std::path::Path;
use structopt::StructOpt;

use config::Config;

fn read_contest(p: &path::PathBuf) -> Option<Contest> {
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
    fn walk_me_recursively(p: &path::PathBuf, contests: &mut Vec<Contest>) {
        if let Ok(paths) = fs::read_dir(p) {
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
    match fs::read_dir(task_dir) {
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
    print!("'{}' …", &password);

    admin.username = Some("admin".into());
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
    //println!("{:?}", opt); // Show in different debug level?

    let mut config = config::read_config_from_file(&opt.configfile);

    if opt.databasefile.is_some() {
        config.database_file = opt.databasefile;
    }
    if config.database_file.is_none() {
        config.database_file = Some(Path::new("medal.db").to_owned())
    }
    if opt.databaseurl.is_some() {
        config.database_url = opt.databaseurl;
    }
    if opt.port.is_some() {
        config.port = opt.port;
    }
    if opt.nocontestscan {
        config.no_contest_scan = Some(true);
    }
    if opt.openbrowser {
        config.open_browser = Some(true)
    }

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
    use std::io::Read;

    fn start_server_and_fn<F>(port: u16, set_user: Option<(String, String)>, f: F)
        where F: FnOnce() {
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
                match test_user.set_password(&user.1) {
                    None => panic!("Set Password did not work correctly.)"),
                    _ => conn.save_session(test_user),
                }
            }

            let mut config = config::read_config_from_file(Path::new("thisfileshoudnotexist"));
            config.port = Some(port);
            let srvr = start_server(conn, config);

            start_tx.send(()).unwrap();

            stop_rx.recv().unwrap();

            srvr.unwrap().close().unwrap();
        });

        // wait for server to start:
        start_rx.recv().unwrap();
        thread::sleep(time::Duration::from_millis(100));
        f();
        stop_tx.send(()).unwrap();
    }

    fn login_for_tests(port: u16, client: &reqwest::Client, username: &str, password: &str) -> reqwest::Response {
        let params = [("username", username), ("password", password)];
        let resp = client.post(&format!("http://localhost:{}/login", port)).form(&params).send().unwrap();
        resp
    }

    fn check_status(resp: &reqwest::Response, expected_status: reqwest::StatusCode) {
        let status = resp.status();
        if status != &expected_status {
            panic!("Status is not (as expexted) {}. Status: {}", expected_status, status)
        };
    }

    #[test]
    fn start_server_and_check_request() {
        start_server_and_fn(8080, None, || {
            let mut resp = reqwest::get("http://localhost:8080").unwrap();
            check_status(&resp, reqwest::StatusCode::Ok);
            let mut content = String::new();
            resp.read_to_string(&mut content).unwrap();
            assert!(content.contains("Jugendwettbewerb Informatik</h1>"));
            assert!(!content.contains("Error"));

            let mut resp = reqwest::get("http://localhost:8080/contest").unwrap();
            check_status(&resp, reqwest::StatusCode::Ok);
            let mut content = String::new();
            resp.read_to_string(&mut content).unwrap();
            assert!(content.contains("<h1>Wettbewerbe</h1>"));
            assert!(!content.contains("Error"));
        })
    }

    #[test]
    fn check_login_wrong_credentials() {
        start_server_and_fn(8081, None, || {
            let client = reqwest::Client::new().unwrap();
            let mut resp = login_for_tests(8081, &client, "nonexistingusername", "wrongpassword");
            check_status(&resp, reqwest::StatusCode::Ok);
            let mut content = String::new();
            resp.read_to_string(&mut content).unwrap();
            assert!(content.contains("<h1>Login</h1>"));
            assert!(content.contains("Login fehlgeschlagen."));
            assert!(!content.contains("Error"));
        })
    }

    #[test]
    fn start_server_and_check_login() {
        start_server_and_fn(8082, Some(("testusr".to_string(), "testpw".to_string())), || {
            let mut client = reqwest::Client::new().unwrap();
            client.redirect(reqwest::RedirectPolicy::custom(|attempt| attempt.stop()));
            let mut resp = login_for_tests(8082, &client, "testusr", "testpw");
            check_status(&resp, reqwest::StatusCode::Found);

            let mut content = String::new();
            resp.read_to_string(&mut content).unwrap();
            assert!(!content.contains("Error"));

            let header = resp.headers();
            let set_cookie = header.get::<reqwest::header::SetCookie>();
            match set_cookie {
                None => panic!("No setCookie."),
                Some(cookie) => {
                    if cookie.len() == 1 {
                        let new_cookie = reqwest::header::Cookie(cookie.to_vec());
                        let mut new_resp = client.get("http://localhost:8082").header(new_cookie).send().unwrap();
                        check_status(&new_resp, reqwest::StatusCode::Ok);

                        let mut new_content = String::new();
                        new_resp.read_to_string(&mut new_content).unwrap();
                        assert!(!content.contains("Error"));
                        assert!(new_content.contains("Eingeloggt als <em>testusr</em>"));
                        assert!(new_content.contains("Jugendwettbewerb Informatik</h1>"));
                    } else {
                        panic!("More than one setCookie.");
                    }
                }
            };
        })
    }

    #[test]
    fn start_server_and_check_logout() {
        start_server_and_fn(8083, Some(("testusr".to_string(), "testpw".to_string())), || {
            let mut client = reqwest::Client::new().unwrap();
            client.redirect(reqwest::RedirectPolicy::custom(|attempt| attempt.stop()));
            let resp = login_for_tests(8083, &client, "testusr", "testpw");
            check_status(&resp, reqwest::StatusCode::Found);

            let header = resp.headers();
            let set_cookie = header.get::<reqwest::header::SetCookie>();
            match set_cookie {
                None => panic!("No setCookie."),
                Some(cookie) => {
                    if cookie.len() == 1 {
                        let new_cookie = reqwest::header::Cookie(cookie.to_vec());
                        let mut new_resp =
                            client.get("http://localhost:8082/logout").header(new_cookie.clone()).send().unwrap();
                        check_status(&new_resp, reqwest::StatusCode::Found);
                        new_resp = client.get("http://localhost:8082").header(new_cookie).send().unwrap();
                        check_status(&new_resp, reqwest::StatusCode::Ok);

                        let mut new_content = String::new();
                        new_resp.read_to_string(&mut new_content).unwrap();
                        assert!(new_content.contains("Benutzername"));
                        assert!(new_content.contains("Passwort"));
                        assert!(new_content.contains("Gruppencode / Teilnahmecode"));
                        assert!(new_content.contains("Jugendwettbewerb Informatik</h1>"));
                    } else {
                        panic!("More than one setCookie.");
                    }
                }
            };
        })
    }

}
