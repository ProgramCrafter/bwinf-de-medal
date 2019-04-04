#[macro_use]
extern crate iron;
#[macro_use]
extern crate router;
#[macro_use]
extern crate serde_derive;

extern crate structopt;
extern crate rusqlite;
extern crate iron_sessionstorage;
extern crate urlencoded;
extern crate time;
extern crate persistent;
extern crate rand;
extern crate mount;
extern crate staticfile;
extern crate handlebars_iron;
extern crate serde_json;
extern crate params;
extern crate reqwest;
extern crate serde_yaml;

use rusqlite::Connection;

mod db_apply_migrations;
mod db_conn_sqlite;
mod db_conn;
mod db_objects;

use functions::SetPassword; // TODO: Refactor, so we don't need to take this from there!
use db_conn::{MedalConnection, MedalObject};

use db_objects::*;

mod webfw_iron;
mod configreader_yaml;

use webfw_iron::start_server;

mod functions;

use std::path;
use std::fs;

use std::path::{Path,PathBuf};
use structopt::StructOpt;

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Config {
    host: Option<String>,
    port: Option<u16>,
    self_url: Option<String>,
    oauth_url: Option<String>,
    oauth_client_id: Option<String>,
    oauth_client_secret: Option<String>,
    oauth_access_token_url: Option<String>,
    oauth_user_data_url: Option<String>,
    database_file: Option<PathBuf>,
}

fn read_config_from_file(file: &Path) -> Config {
    use std::io::Read;

    println!("Reading configuration file '{}'", file.to_str().unwrap_or("<Encoding error>"));

    let mut config : Config = if let Ok(mut file) = fs::File::open(file) {
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        serde_json::from_str(&contents).unwrap()
    } else {
        println!("Configuration file '{}' not found.", file.to_str().unwrap_or("<Encoding error>"));
        Default::default()
    };

    if config.host.is_none() {config.host = Some("[::]".to_string())}
    if config.port.is_none() {config.port = Some(8080)}
    if config.self_url.is_none() {config.self_url = Some("http://localhost:8080".to_string())}

    println!("OAuth providers will be told to redirect to {}", config.self_url.as_ref().unwrap());

    config
}

#[derive(StructOpt, Debug)]
#[structopt()]
struct Opt {
    /// Config file to use (default: 'config.json')
    #[structopt(short = "c", long = "config", default_value = "config.json", parse(from_os_str))]
    configfile: PathBuf,

    /// Database file to use (default: from config file or 'medal.db')
    #[structopt(short = "d", long = "database", parse(from_os_str))]
    databasefile: Option<PathBuf>,

    /// Port to listen on (default: from config file or 8080)
    #[structopt(short = "p", long = "port")]
    port: Option<u16>,

    /// Reset password of admin user (user_id=1)
    #[structopt(short = "a", long = "reset-admin-pw")]
    resetadminpw: bool,
}



fn read_contest(p: &path::PathBuf) -> Option<Contest> {
    use std::fs::File;
    use std::io::Read;

    let mut file = File::open(p).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();

    configreader_yaml::parse_yaml(&contents, p.file_name().to_owned()?.to_str()?, &format!("{}/", p.parent().unwrap().to_str()?))
}

fn get_all_contest_info(task_dir: &str) -> Vec<Contest> {
    fn walk_me_recursively(p: &path::PathBuf, contests: &mut Vec<Contest>) {
        match fs::read_dir(p) {
            Ok(paths) => for path in paths {
                let p = path.unwrap().path();
                walk_me_recursively(&p, contests);
            },
            _ => (),
        }

        if p.file_name().unwrap().to_string_lossy().to_string().ends_with(".yaml") {
            match read_contest(p) {
                Some(contest) => contests.push(contest),
                _ => (),
            }
        };
    };


    let mut contests = Vec::new();
    match fs::read_dir(task_dir) {
        Err(why) => println!("Error opening tasks directory! {:?}", why.kind()),
        Ok(paths) => for path in paths {
            walk_me_recursively(&path.unwrap().path(), &mut contests);
        },
    };

    contests
}

fn refresh_all_contests(conn : &mut Connection) {
    let v = get_all_contest_info("tasks/");

    for mut contest_info in v {
        contest_info.save(conn);
    }
}

fn add_admin_user(conn: &mut Connection, resetpw: bool) {
    let mut admin = match conn.get_user_by_id(1) {
        None => {
            print!("New Database. Creating new admin user with credentials 'admin':");
            conn.new_session()

        },
        Some(user) => {
            if !resetpw {
                return
            }
            print!("Request to reset admin password. Set credentials 'admin':");
            user
        }
    };

    use rand::{thread_rng, Rng, distributions::Alphanumeric};

    let password: String = thread_rng().sample_iter(&Alphanumeric)
        .filter(|x| {let x = *x; !(x == 'l' || x == 'I' || x == '1' || x == 'O' || x == 'o' || x == '0')})
        .take(8).collect();
    print!("'{}' …", &password);

    admin.username = Some("admin".into());
    match admin.set_password(&password) {
        None => println!("FAILED! (Password hashing error)"),
        _ => {
            conn.save_session(admin);
            println!("Done");
        }
    }
}

fn main() {
    let opt = Opt::from_args();
    //println!("{:?}", opt); // Show in different debug level?

    let mut config = read_config_from_file(&opt.configfile);

    if opt.databasefile.is_some() { config.database_file = opt.databasefile; }
    if opt.port.is_some() { config.port = opt.port; }

    let mut conn = match config.database_file {
        Some(ref path) => {println!("Using database file {}", &path.to_str().unwrap_or("<unprintable filename>"));  Connection::create(path)},
        None => {println!("Using default database file ./medal.db"); Connection::create(&Path::new("medal.db"))},
    };

    db_apply_migrations::test(&mut conn);

    refresh_all_contests(&mut conn);

    add_admin_user(&mut conn, opt.resetadminpw);

    match start_server(conn, config) {
        Ok(_) => println!("Server started"),
        Err(_) => println!("Error on server start …")
    };

    println!("Could not run server. Is the port already in use?");
}



#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    fn start_server_and_fn<F>(port: u16, f: F) where F: FnOnce() {
        use std::{thread, time};
        use std::sync::mpsc::channel;
        let (start_tx, start_rx) = channel();
        let (stop_tx, stop_rx) = channel();

        thread::spawn(move || {
            let mut conn = Connection::open_in_memory().unwrap();
            db_apply_migrations::test(&mut conn);
            let mut config = read_config_from_file(Path::new("thisfileshoudnotexist"));
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

    #[test]
    fn start_server_and_check_request() {
        start_server_and_fn(8080, ||{
            let mut resp = reqwest::get("http://localhost:8080").unwrap();
            assert!(resp.status().is_success());
            let mut content = String::new();
            resp.read_to_string(&mut content);
            assert!(content.contains("<h1>Jugendwettbewerb Informatik</h1>"));
            assert!(!content.contains("Error"));


            let mut resp = reqwest::get("http://localhost:8080/contest").unwrap();
            assert!(resp.status().is_success());
            let mut content = String::new();
            resp.read_to_string(&mut content);
            assert!(content.contains("<h1>Wettbewerbe</h1>"));
            assert!(!content.contains("Error"));
        })
    }

    #[test]
    fn check_login_wrong_credentials() {
        start_server_and_fn(8081, ||{
            let params = [("username", "nonexistingusername"), ("password", "wrongpassword")];
            let client = reqwest::Client::new().unwrap();
            let mut resp = client.post("http://localhost:8081/login")
                .form(&params).send().unwrap();
            let mut content = String::new();
            resp.read_to_string(&mut content);
            assert!(content.contains("<h1>Login</h1>"));
            assert!(content.contains("Login fehlgeschlagen."));
            assert!(!content.contains("Error"));
        })
    }
}
