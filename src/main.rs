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

extern crate csv;
extern crate handlebars_iron;
extern crate iron_sessionstorage;
extern crate mount;
extern crate params;
extern crate persistent;
extern crate rand;
extern crate reqwest;
extern crate serde_json;
extern crate serde_yaml;
extern crate sha2;
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

use std::path::Path;

fn read_contest(p: &Path) -> Option<Contest> {
    use std::fs::File;
    use std::io::Read;

    let mut file = File::open(p).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).ok()?;

    contestreader_yaml::parse_yaml(&contents,
                                   p.file_name().to_owned()?.to_str()?,
                                   &format!("{}/", p.parent().unwrap().to_str()?))
}

fn get_all_contest_info(task_dir: &str) -> Vec<Contest> {
    fn walk_me_recursively(p: &Path, contests: &mut Vec<Contest>) {
        if let Ok(paths) = std::fs::read_dir(p) {
            print!("…");
            use std::io::Write;
            std::io::stdout().flush().unwrap();
            let mut paths: Vec<_> = paths.filter_map(|r| r.ok()).collect();
            paths.sort_by_key(|dir| dir.path());
            for path in paths {
                let p = path.path();
                walk_me_recursively(&p, contests);
            }
        }

        if p.file_name().unwrap().to_string_lossy().to_string().ends_with(".yaml") {
            read_contest(p).map(|contest| contests.push(contest));
        };
    }

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

    let password = helpers::make_unambiguous_code(8);
    print!("'{}', ", &password);

    let logincode = helpers::make_unambiguous_code_prefix(8, "a");
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
    opt.port.map(|x| config.port = Some(x));
    config.no_contest_scan = if opt.nocontestscan { Some(true) } else { config.no_contest_scan };
    config.open_browser = if opt.openbrowser { Some(true) } else { config.open_browser };
    config.disable_results_page = if opt.disableresultspage { Some(true) } else { config.disable_results_page };
    config.enable_password_login = if opt.enablepasswordlogin { Some(true) } else { config.enable_password_login };

    // Use default database file if none set
    config.database_file.get_or_insert(Path::new("medal.db").to_owned());

    #[cfg(feature = "debug")]
    println!("Using config: {:#?}", config);

    #[cfg(feature = "postgres")]
    {
        if let Some(url) = config.database_url.clone() {
            #[cfg(feature = "debug")]
            print!("Using database {} … ", &url);
            #[cfg(not(feature = "debug"))]
            {
                let (begin_middle, end) = url.split_at(url.find('@').unwrap_or(0));
                let (begin, _middle) = begin_middle.split_at(begin_middle.rfind(':').unwrap_or(0));
                print!("Using database {}:***{} … ", begin, end);
            }
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
mod tests;
