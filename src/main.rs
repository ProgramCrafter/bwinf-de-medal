#[macro_use]
extern crate iron;
#[macro_use]
extern crate router;
#[macro_use]
extern crate serde_derive;

#[macro_use]
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

    println!("Reading Config file '{}'", file.to_str().unwrap_or("<Encoding error>"));
    
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

    println!("I will ask OAuth-providers to redirect to {}", config.self_url.as_ref().unwrap());
    
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
}



fn read_contest(p: &path::PathBuf) -> Option<Contest> {
    use std::fs::File;
    use std::io::Read;
    println!("Try to read some file …");

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

fn add_admin_user(conn: &mut Connection) {
    if conn.get_user_by_id(1).is_none() {

        print!("New Database. Creating new admin user with credentials 'admin':'test' … ");
        let mut admin = conn.new_session();
        admin.username = Some("admin".into());
        admin.password = Some("test".into());
        admin.salt = Some("".into());
        conn.save_session(admin);
        println!("Done");
    }
}

fn main() {
    let opt = Opt::from_args();
    println!("{:?}", opt);
    
    let mut config = read_config_from_file(&opt.configfile);

    if opt.databasefile.is_some() { config.database_file = opt.databasefile; }
    if opt.port.is_some() { config.port = opt.port; }
    
    let mut conn = match config.database_file {
        Some(ref path) => {println!("Using database file {}", &path.to_str().unwrap_or("<unprintable filename>"));  Connection::create(path)},
        None => {println!("Using default database file ./medal.db"); Connection::create(&Path::new("medal.db"))},
    };
        
    db_apply_migrations::test(&mut conn);

    refresh_all_contests(&mut conn);
        
    println!("Hello, world!");

    let contest = conn.get_contest_by_id_complete(1);
    add_admin_user(&mut conn);

    println!("Contest {}", contest.name);
    
    for taskgroup in contest.taskgroups {
        print!("  Task {}: ", taskgroup.name);
        for task in taskgroup.tasks {
            print!("{} ({}) ", task.stars, task.location);
        }
        println!("");
    }
    
    start_server(conn, config);

    println!("Could not run server. Is the port already in use?");
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_server_and_check_request() {
        use std::{thread, time};

        
        let mut conn = Connection::open_in_memory().unwrap();
        db_apply_migrations::test(&mut conn);

        // add contests / tasks here

        use std::sync::{Arc, Mutex, Condvar};
        let pair = Arc::new((Mutex::new(false), Condvar::new()));
        let pair_ = pair.clone();

        let mut config = read_config_from_file(Path::new("thisfileshoudnotexist"));
        
        let srvr = start_server(conn, config);

        thread::spawn(move || {
            // wait for server to start:
            thread::sleep(time::Duration::from_millis(100));

            use std::io::Read;

            let mut resp = reqwest::get("http://localhost:8080").unwrap();
            assert!(resp.status().is_success());

            let mut content = String::new();
            resp.read_to_string(&mut content);
            assert!(content.contains("<h1>Jugendwettbewerb Informatik</h1>"));
            assert!(!content.contains("Error"));
            
            let &(ref lock, ref cvar) = &*pair_;
            let mut should_exit = lock.lock().unwrap();
            *should_exit = true;
            cvar.notify_one();
            //fs::copy("foo.txt", "bar.txt").unwrap();
        });
        
        // Copied from docs
        let &(ref lock, ref cvar) = &*pair;
        let mut should_exit = lock.lock().unwrap();
        while !*should_exit {
            should_exit = cvar.wait(should_exit).unwrap();
        }

        srvr.unwrap().close().unwrap();
        
        assert!(true);
    }
}
