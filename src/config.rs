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
    pub template_params: Option<::std::collections::BTreeMap<String, String>>,
}

#[derive(StructOpt, Debug)]
#[structopt()]
pub struct Opt {
    /// Config file to use (default: 'config.json')
    #[structopt(short = "c", long = "config", default_value = "config.json", parse(from_os_str))]
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

    /// Automatically open medal in the default browser
    #[structopt(short = "P", long = "passwordlogin")]
    pub enablepasswordlogin: bool,

    /// Teacher page in task directory
    #[structopt(short = "t", long = "teacherpage")]
    pub teacherpage: Option<String>,
}

pub fn read_config_from_file(file: &Path) -> Config {
    use std::io::Read;

    println!("Reading configuration file '{}'", file.to_str().unwrap_or("<Encoding error>"));

    let mut config: Config = if let Ok(mut file) = std::fs::File::open(file) {
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        serde_json::from_str(&contents).unwrap()
    } else {
        println!("Configuration file '{}' not found.", file.to_str().unwrap_or("<Encoding error>"));
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

    println!("OAuth providers will be told to redirect to {}", config.self_url.as_ref().unwrap());

    config
}
