//#![feature(extern_prelude)]

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

#[derive(Serialize, Deserialize)]
pub struct Config {
    oauth_client_id: Option<String>,
    oauth_client_secret: Option<String>,
    oauth_access_token_url: Option<String>,
    oauth_user_data_url: Option<String>,
    database_file: Option<PathBuf>,
}

fn read_config_from_file(file: &Path) -> Config {
    use std::io::Read;
    
    if let Ok(mut file) = fs::File::open(file) {
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        serde_json::from_str(&contents).unwrap()
    } else {
        println!("Configuration file '{}' not found.", file.to_str().unwrap_or("<Encoding error>"));
        Config {
            oauth_client_id: None,
            oauth_client_secret: None,
            oauth_access_token_url: None,
            oauth_user_data_url: None,
            database_file: None,
        }
    }
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
}



fn read_contest(p: &path::PathBuf) -> Option<Contest> {
    use std::fs::File;
    use std::io::Read;
    println!("Try to read some file …");

    let mut file = File::open(p).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();
    
    configreader_yaml::parse_yaml(&contents, p.file_name().to_owned()?.to_str()?, &format!("{}/", p.parent().unwrap().to_str()?)) // unwrap_or("./")?
    /*let mut contest = Contest::new("./".to_string(), "blub.json".to_string(), "Wettbewerb IX".to_string(), 45, true, None, None);
    let mut taskgroup = Taskgroup::new("Lustige Aufgabe".to_string());
    let mut task = Task::new("blub".to_string(), 1);
    taskgroup.tasks.push(task);
    let mut task = Task::new("blub2".to_string(), 4);
    taskgroup.tasks.push(task);  
    contest.taskgroups.push(taskgroup);
    let mut taskgroup = Taskgroup::new("Lustige Aufgabe3".to_string());
    let mut task = Task::new("blub3".to_string(), 2);
    taskgroup.tasks.push(task);
    let mut task = Task::new("blub4".to_string(), 3);
    taskgroup.tasks.push(task);  
    contest.taskgroups.push(taskgroup);

    Some(contest)*/
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

fn main() {
    let opt = Opt::from_args();
    println!("{:?}", opt);
    
    let mut config = read_config_from_file(&opt.configfile);

    if opt.databasefile.is_some() { config.database_file = opt.databasefile; }
    //if config.database_file.is_none() { config.database_file = Some(.to_path_buf()); }
    
    let mut conn = match config.database_file {
        Some(ref path) => Connection::create(path),
        None => Connection::create(&Path::new("medal.db")),
    };
        
    db_apply_migrations::test(&mut conn);

    refresh_all_contests(&mut conn);
        
    println!("Hello, world!");

    let contest = conn.get_contest_by_id_complete(1);

    println!("Contest {}", contest.name);
    
    for taskgroup in contest.taskgroups {
        print!("  Task {}: ", taskgroup.name);
        for task in taskgroup.tasks {
            print!("{} ({}) ", task.stars, task.location);
        }
        println!("");
    }

    start_server(conn, config);

    println!("Server started.");
}
