/*  medal                                                                                                            *\
 *  Copyright (C) 2022  Bundesweite Informatikwettbewerbe, Robert Czechowski                                         *
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

use std::path::{Path, PathBuf};

use structopt::StructOpt;

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct OauthProvider {
    pub provider_id: String,
    pub medal_oauth_type: String,
    pub url: String,
    pub client_id: String,
    pub client_secret: String,
    pub access_token_url: String,
    pub user_data_url: String,
    pub school_data_url: Option<String>,
    pub school_data_secret: Option<String>,
    pub allow_teacher_login_without_school: Option<bool>,
    pub login_link_text: String,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct Config {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub self_url: Option<String>,
    pub oauth_providers: Option<Vec<OauthProvider>>,
    pub database_file: Option<PathBuf>,
    pub database_url: Option<String>,
    pub template: Option<String>,
    pub no_contest_scan: Option<bool>,
    pub open_browser: Option<bool>,
    pub cookie_signing_secret: Option<String>,
    pub disable_results_page: Option<bool>,
    pub enable_password_login: Option<bool>,
    pub require_sex: Option<bool>,
    pub allow_sex_na: Option<bool>,
    pub allow_sex_diverse: Option<bool>,
    pub allow_sex_other: Option<bool>,
    pub dbstatus_secret: Option<String>,
    pub template_params: Option<::std::collections::BTreeMap<String, serde_json::Value>>,
    pub only_contest_scan: Option<bool>,
    pub reset_admin_pw: Option<bool>,
    pub log_timing: Option<bool>,
    pub auto_save_interval: Option<u64>,
}

#[derive(StructOpt, Debug)]
#[structopt()]
struct Opt {
    /// Config file to use (default: 'config.json')
    #[structopt(short = "c", long = "config", default_value = "config.yaml", parse(from_os_str))]
    pub configfile: PathBuf,

    /// Database file to use (default: from config file or 'medal.db')
    #[structopt(short = "d", long = "database", parse(from_os_str))]
    pub databasefile: Option<PathBuf>,

    /// Database file to use (default: from config file or 'medal.db')
    #[structopt(short = "D", long = "databaseurl")]
    pub databaseurl: Option<String>,

    /// Port to listen on (default: from config file or 8080)
    #[structopt(short = "p", long = "port")]
    pub port: Option<u16>,

    /// Teacher page in task directory
    #[structopt(short = "t", long = "template")]
    pub template: Option<String>,

    /// Reset password of admin user (user_id=1)
    #[structopt(short = "a", long = "reset-admin-pw")]
    pub resetadminpw: bool,

    /// Run medal without scanning for contests
    #[structopt(short = "S", long = "no-contest-scan")]
    pub nocontestscan: bool,

    /// Scan for contests without starting medal
    #[structopt(short = "s", long = "only-contest-scan")]
    pub onlycontestscan: bool,

    /// Automatically open medal in the default browser
    #[structopt(short = "b", long = "browser")]
    pub openbrowser: bool,

    /// Disable results page to reduce load on the server
    #[structopt(long = "disable-results-page")]
    pub disableresultspage: bool,

    /// Enable the login with username and password
    #[structopt(short = "P", long = "passwordlogin")]
    pub enablepasswordlogin: bool,

    /// Teacher page in task directory
    #[structopt(short = "T", long = "teacherpage")]
    pub teacherpage: Option<String>,

    /// Log response time of every request
    #[structopt(long = "log-timing")]
    pub logtiming: bool,

    /// Auto save interval in seconds (defaults to 10)
    #[structopt(long = "auto-save-interval")]
    pub autosaveinterval: Option<u64>,
}

enum FileType {
    Json,
    Yaml,
}

pub fn read_config_from_file(file: &Path) -> Config {
    use std::io::Read;

    let file_type = match file.extension().map(|e| e.to_str().unwrap_or("<Encoding error>")) {
        Some("yaml") | Some("YAML") => FileType::Yaml,
        Some("json") | Some("JSON") => FileType::Json,
        Some(ext) => panic!("Config file has unknown file extension `{}` (supported types are YAML and JSON).", ext),
        None => panic!("Config file has no file extension (supported types are YAML and JSON)."),
    };

    println!("Reading configuration file '{}'", file.to_str().unwrap_or("<Encoding error>"));

    let mut config: Config = if let Ok(mut file) = std::fs::File::open(file) {
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        match file_type {
            FileType::Json => serde_json::from_str(&contents).unwrap(),
            FileType::Yaml => serde_yaml::from_str(&contents).unwrap(),
        }
    } else {
        println!("Configuration file '{}' not found. Using default configuration.",
                 file.to_str().unwrap_or("<Encoding error>"));
        Default::default()
    };

    if let Some(ref oap) = config.oauth_providers {
        println!("OAuth providers:");
        for oap in oap {
            println!("  * {}", oap.provider_id);
        }
    }

    if config.host.is_none() {
        config.host = Some("[::]".to_string())
    }
    if config.port.is_none() {
        config.port = Some(8080)
    }
    if config.self_url.is_none() {
        config.self_url = Some("http://localhost:8080".to_string())
    }
    if config.template.is_none() {
        config.template = Some("default".to_string())
    }
    if config.no_contest_scan.is_none() {
        config.no_contest_scan = Some(false)
    }
    if config.open_browser.is_none() {
        config.open_browser = Some(false)
    }
    if config.enable_password_login.is_none() {
        config.enable_password_login = Some(false)
    }
    if config.auto_save_interval.is_none() {
        config.auto_save_interval = Some(10)
    }

    println!("OAuth providers will be told to redirect to {}", config.self_url.as_ref().unwrap());

    config
}

fn merge_value<T>(into: &mut Option<T>, from: Option<T>) { from.map(|x| *into = Some(x)); }

fn merge_flag(into: &mut Option<bool>, from: bool) {
    if from {
        *into = Some(true);
    }
}

pub fn get_config() -> Config {
    let opt = Opt::from_args();

    #[cfg(feature = "debug")]
    println!("Options: {:#?}", opt);

    let mut config = read_config_from_file(&opt.configfile);

    #[cfg(feature = "debug")]
    println!("Config: {:#?}", config);

    // Let options override config values
    merge_value(&mut config.database_file, opt.databasefile);
    merge_value(&mut config.database_url, opt.databaseurl);
    merge_value(&mut config.port, opt.port);
    merge_value(&mut config.template, opt.template);
    merge_value(&mut config.auto_save_interval, opt.autosaveinterval);

    merge_flag(&mut config.no_contest_scan, opt.nocontestscan);
    merge_flag(&mut config.open_browser, opt.openbrowser);
    merge_flag(&mut config.disable_results_page, opt.disableresultspage);
    merge_flag(&mut config.enable_password_login, opt.enablepasswordlogin);
    merge_flag(&mut config.only_contest_scan, opt.onlycontestscan);
    merge_flag(&mut config.reset_admin_pw, opt.resetadminpw);
    merge_flag(&mut config.log_timing, opt.logtiming);

    if let Some(template_params) = &mut config.template_params {
        if let Some(teacherpage) = opt.teacherpage {
            template_params.insert("teacher_page".to_string(), teacherpage.into());
        }
    } else if let Some(teacherpage) = opt.teacherpage {
        let mut template_params = ::std::collections::BTreeMap::<String, serde_json::Value>::new();
        template_params.insert("teacher_page".to_string(), teacherpage.into());
        config.template_params = Some(template_params);
    }

    // Use default database file if none set
    config.database_file.get_or_insert(Path::new("medal.db").to_owned());

    config
}
