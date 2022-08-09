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

use std::path::Path;

pub use handlebars_iron::handlebars::to_json;
use handlebars_iron::{DirectorySource, HandlebarsEngine, Template};
use iron;
use iron::mime::Mime;
use iron::modifiers::Redirect;
use iron::modifiers::RedirectRaw;
use iron::prelude::*;
use iron::{status, AfterMiddleware, AroundMiddleware, Handler};
use iron_sessionstorage;
use iron_sessionstorage::backends::SignedCookieBackend;
use iron_sessionstorage::traits::*;
use iron_sessionstorage::SessionStorage;
use mount::Mount;
use persistent::{Read, Write};
use reqwest;
use router::Router;
use staticfile::Static;
use urlencoded::{UrlEncodedBody, UrlEncodedQuery};

#[cfg(feature = "debug")]
use iron::BeforeMiddleware;

use config::{Config, OauthProvider};
use core;
use db_conn::MedalConnection;
use iron::typemap::Key;
pub use serde_json::value as json_val;

#[cfg(feature = "signup")]
use db_conn::SignupResult;

static TASK_DIR: &str = "tasks";

macro_rules! mime {
    ($top:tt / $sub:tt) => (
        mime!($top / $sub;)
    );

    ($top:tt / $sub:tt ; $($attr:tt = $val:tt),*) => (
        iron::mime::Mime(
            iron::mime::TopLevel::$top,
            iron::mime::SubLevel::$sub,
            vec![ $((Attr::$attr,Value::$val)),* ]
        )
    );
}

macro_rules! with_conn {
    ( $x:expr , $c:ident, $r:expr , $($y:expr),* ) => {
        {
            let mutex = $r.get::<Write<SharedDatabaseConnection<$c>>>().unwrap();
            let conn = mutex.lock().unwrap_or_else(|e| e.into_inner());
            $x(&*conn, $($y),*)
        }
    };
}

macro_rules! template_ok {
    ( $x:expr ) => {{
        let (template, data) = $x;

        let mut resp = Response::new();
        resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
        Ok(resp)
    }};
}

/** Log error messages on commandline */
struct ErrorReporter;
impl AfterMiddleware for ErrorReporter {
    fn catch(&self, req: &mut Request, err: IronError) -> IronResult<Response> {
        if err.response.status != Some(status::Found) || cfg!(feature = "debug") {
            println!("{}    {} {}", err, req.method, req.url);
        }
        Err(err)
    }
}

/** Show error messages to users */
struct ErrorShower;
impl AfterMiddleware for ErrorShower {
    fn catch(&self, _: &mut Request, err: IronError) -> IronResult<Response> {
        let IronError { error, response } = err;
        if response.body.is_none() {
            Ok(match response.status {
                Some(s) => {
                    let n = s.to_u16();
                    if (400..=599).contains(&n) {
                        response.set((mime!(Text / Html),
                                      format!("<h1>{} {}</h1>", n, s.canonical_reason().unwrap_or("(Unknown error)"))))
                    } else {
                        response
                    }
                }
                _ => response,
            })
        } else {
            Err(IronError { error, response })
        }
    }
}

#[derive(Debug)]
struct SessionToken {
    token: String,
}
impl iron_sessionstorage::Value for SessionToken {
    fn get_key() -> &'static str { "medal_session" }
    fn into_raw(self) -> String { self.token }
    fn from_raw(value: String) -> Option<Self> {
        if value.is_empty() {
            None
        } else {
            Some(SessionToken { token: value })
        }
    }
}

pub struct RequestTimeLogger {}

impl AroundMiddleware for RequestTimeLogger {
    fn around(self, handler: Box<dyn Handler>) -> Box<dyn Handler> {
        use std::time::{Duration, Instant};

        Box::new(move |req: &mut Request| -> IronResult<Response> {
            // Set thresholds
            let (threshold, threshold_critical) = match req.url.path().get(0) {
                Some(&"save") => (Duration::from_millis(80), Duration::from_millis(120)),
                Some(&"contest") => (Duration::from_millis(80), Duration::from_millis(120)),
                Some(&"oauth") => (Duration::from_millis(800), Duration::from_millis(3200)),
                _ => (Duration::from_millis(20), Duration::from_millis(80)),
            };

            // Begin measurement
            let start = Instant::now();

            // Get config value
            let logtiming = {
                let config = req.get::<Read<SharedConfiguration>>().unwrap();
                config.log_timing.unwrap_or(false)
            };

            // Process request
            let res = handler.handle(req);

            // End measurement
            let duration = start.elapsed();

            if logtiming {
                println!("t:\t{:?}\t{}\t{}", duration, req.method, req.url);
            } else if duration > threshold_critical {
                println!("Request took MUCH too long ({:?})    {} {}", duration, req.method, req.url);
            } else if duration > threshold {
                println!("Request took too long ({:?})    {} {}", duration, req.method, req.url);
            }

            res
        })
    }
}

pub struct CookieDistributor {}

impl AroundMiddleware for CookieDistributor {
    fn around(self, handler: Box<dyn Handler>) -> Box<dyn Handler> {
        use rand::{distributions::Alphanumeric, thread_rng, Rng};

        Box::new(move |req: &mut Request| -> IronResult<Response> {
            if req.session().get::<SessionToken>().expect("blub...").is_none() {
                let session_token: String = thread_rng().sample_iter(&Alphanumeric).take(10).collect();
                req.session().set(SessionToken { token: session_token }).unwrap();
            }
            handler.handle(req)
        })
    }
}

#[cfg(feature = "debug")]
pub struct RequestLogger {}

#[cfg(feature = "debug")]
impl BeforeMiddleware for RequestLogger {
    fn before(&self, req: &mut Request) -> IronResult<()> {
        println!("{}: {}", req.method, req.url);

        Ok(())
    }
}

#[derive(Debug)]
struct SessionError {
    message: String,
}
impl ::std::error::Error for SessionError {
    fn description(&self) -> &str { &self.message }
}

impl ::std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result { write!(f, "{}", self.message) }
}

trait RequestSession {
    fn get_session_token(&mut self) -> Option<String>;
    fn require_session_token(&mut self) -> IronResult<String>;
    fn expect_session_token(&mut self) -> IronResult<String>;
}

impl<'a, 'b> RequestSession for Request<'a, 'b> {
    fn get_session_token(&mut self) -> Option<String> {
        let session_token = self.session().get::<SessionToken>().unwrap();
        (|st: Option<SessionToken>| -> Option<String> { Some(st?.token) })(session_token)
    }

    fn require_session_token(&mut self) -> IronResult<String> {
        match self.session().get::<SessionToken>().unwrap() {
            Some(SessionToken { token: session }) => Ok(session),
            _ => {
                use rand::{distributions::Alphanumeric, thread_rng, Rng};

                let new_session_key: String = thread_rng().sample_iter(&Alphanumeric).take(28).collect();
                self.session().set(SessionToken { token: new_session_key }).unwrap();
                Err(IronError {
                    error: Box::new(SessionError {
                        message: "No valid session found, redirecting to cookie page".to_string(),
                    }),
                    response: Response::with((
                        status::Found,
                        RedirectRaw(format!("/cookie?{}", self.url.path().join("/"))),
                    )),
                })
            }
        }
    }

    fn expect_session_token(&mut self) -> IronResult<String> {
        match self.session().get::<SessionToken>().unwrap() {
            Some(SessionToken { token: session }) => Ok(session),
            _ => Err(IronError { error: Box::new(SessionError { message:
                                                                    "No valid session found, access denied".to_string() }),
                                 response: Response::with(status::Forbidden) }),
        }
    }
}

trait RequestRouterParam {
    fn get_str(&mut self, key: &str) -> Option<String>;
    fn get_int<T: ::std::str::FromStr>(&mut self, key: &str) -> Option<T>;
    fn expect_int<T: ::std::str::FromStr>(&mut self, key: &str) -> IronResult<T>;
    fn expect_str(&mut self, key: &str) -> IronResult<String>;
}

impl<'a, 'b> RequestRouterParam for Request<'a, 'b> {
    fn get_str(&mut self, key: &str) -> Option<String> { Some(self.extensions.get::<Router>()?.find(key)?.to_owned()) }

    fn get_int<T: ::std::str::FromStr>(&mut self, key: &str) -> Option<T> {
        self.extensions.get::<Router>()?.find(key)?.parse::<T>().ok()
    }

    fn expect_int<T: ::std::str::FromStr>(&mut self, key: &str) -> IronResult<T> {
        match self.get_int::<T>(key) {
            Some(i) => Ok(i),
            _ => Err(IronError { error: Box::new(SessionError { message:
                                                                    "No valid routing parameter".to_string() }),
                                 response: Response::with(status::Forbidden) }),
        }
    }

    fn expect_str(&mut self, key: &str) -> IronResult<String> {
        match self.get_str(key) {
            Some(s) => Ok(s),
            _ => Err(IronError { error: Box::new(SessionError { message:
                                                                    "Routing parameter missing".to_string() }),
                                 response: Response::with(status::Forbidden) }),
        }
    }
}

struct AugMedalError<'c, 'a: 'c, 'b: 'c + 'a>(core::MedalError, &'c mut Request<'a, 'b>);

impl<'c, 'a, 'b> From<AugMedalError<'c, 'a, 'b>> for IronError {
    fn from(AugMedalError(me, req): AugMedalError<'c, 'a, 'b>) -> Self {
        match me {
            core::MedalError::NotLoggedIn => {
                IronError { error: Box::new(SessionError { message:
                                                               "Not Logged in, redirecting to login page".to_string() }),
                            response: Response::with((status::Found,
                                                      RedirectRaw(format!("/login?{}", req.url.path().join("/"))))) }
            }
            core::MedalError::AccessDenied => IronError { error: Box::new(SessionError { message:
                                                                                             "Access denied".to_string() }),
                                                          response: Response::with(status::Unauthorized) },
            core::MedalError::UnknownId => IronError { error: Box::new(SessionError { message:
                                                                                      "Not found".to_string() }),
                                                       response: Response::with(status::NotFound) },
            core::MedalError::CsrfCheckFailed => IronError { error: Box::new(SessionError { message:
                                                                                                "CSRF Error".to_string() }),
                                                             response: Response::with(status::Forbidden) },
            core::MedalError::SessionTimeout => {
                IronError { error: Box::new(SessionError { message: "Session timed out".to_string() }),
                            response: Response::with(status::Forbidden) }
            }
            core::MedalError::DatabaseError => {
                IronError { error: Box::new(SessionError { message: "Database Error".to_string() }),
                            response: Response::with(status::InternalServerError) }
            }
            core::MedalError::ConfigurationError => {
                IronError { error: Box::new(SessionError { message: "Server misconfiguration. Please contact an administrator!".to_string() }),
                            response: Response::with(status::InternalServerError) }
            }
            core::MedalError::DatabaseConnectionError => {
                IronError { error: Box::new(SessionError { message: "Database Connection Error".to_string() }),
                            response: Response::with(status::InternalServerError) }
            }
            core::MedalError::PasswordHashingError => {
                IronError { error: Box::new(SessionError { message: "Error hashing the passwords".to_string() }),
                            response: Response::with(status::InternalServerError) }
            }
            core::MedalError::UnmatchedPasswords => {
                IronError { error: Box::new(SessionError { message:
                                                               "The two passwords did not match.".to_string() }),
                            response: Response::with(status::Forbidden) }
            }
            core::MedalError::NotFound => IronError { error: Box::new(SessionError { message:
                                                                                         "Not found".to_string() }),
                                                      response: Response::with(status::NotFound) },
            core::MedalError::AccountIncomplete => IronError { error: Box::new(SessionError { message:
                                                                                              "Account incomplete".to_string() }),
                                                               response: Response::with((status::Found,
                                                                                         Redirect(iron::Url::parse(&format!("{}?status=firstlogin",
                                                                                                                            &url_for!(req, "profile"))).unwrap()))) },
            core::MedalError::OauthError(errstr) => {
                IronError { error: Box::new(SessionError { message: format!("Access denied (Error {})", errstr) }),
                            response: Response::with(status::Unauthorized) }
            }
        }
    }
}

trait RequestAugmentMedalError<'c, 'a: 'c, 'b: 'c + 'a, R> {
    fn aug(self, req: &'c mut Request<'a, 'b>) -> Result<R, AugMedalError<'c, 'a, 'b>>;
}
impl<'c, 'a: 'c, 'b: 'c + 'a, T> RequestAugmentMedalError<'c, 'a, 'b, T> for Result<T, core::MedalError> {
    fn aug(self, req: &'c mut Request<'a, 'b>) -> Result<T, AugMedalError<'c, 'a, 'b>> {
        self.map_err(move |me| AugMedalError(me, req))
    }
}

fn login_info(config: &Config) -> core::LoginInfo {
    core::LoginInfo { password_login: config.enable_password_login == Some(true),
                      self_url: config.self_url.clone(),
                      oauth_providers: config.oauth_providers.clone() }
}

fn greet_personal<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let session_token = req.get_session_token();
    // hier ggf. Daten aus dem Request holen

    let config = req.get::<Read<SharedConfiguration>>().unwrap();
    let (template, mut data) = with_conn![core::index, C, req, session_token, login_info(&config)].aug(req)?;

    data.insert("config".to_string(), to_json(&config.template_params));

    // Antwort erstellen und zurücksenden
    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}

fn dbstatus<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let config = req.get::<Read<SharedConfiguration>>().unwrap();
    let query_string = req.url.query().map(|s| s.to_string());

    let status = with_conn![core::status, C, req, config.dbstatus_secret.clone(), query_string].aug(req)?;

    let mut resp = Response::new();
    resp.set_mut(status).set_mut(status::Ok);
    Ok(resp)
}

fn debug<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let session_token = req.get_session_token();

    let (template, data) = with_conn![core::debug, C, req, session_token];

    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}

fn debug_new_token<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let session_token = req.get_session_token();

    #[cfg(feature = "debug")]
    println!("Logging out session {:?}", session_token);

    with_conn![core::logout, C, req, session_token];

    Ok(Response::with((status::Found, Redirect(url_for!(req, "debug")))))
}

fn debug_logout<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let session_token = req.get_session_token();

    #[cfg(feature = "debug")]
    println!("Logging out session {:?}", session_token);

    with_conn![core::logout, C, req, session_token];

    Ok(Response::with((status::Found, Redirect(url_for!(req, "debug")))))
}

fn debug_create_session<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let session_token = req.get_session_token();

    with_conn![core::debug_create_session, C, req, session_token];

    Ok(Response::with((status::Found, Redirect(url_for!(req, "debug")))))
}

fn contests<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let session_token = req.require_session_token()?;
    let query_string = req.url.query().unwrap_or("").to_string();

    // TODO: Move to core::* ?
    let visibility = if query_string.contains("open") {
        core::ContestVisibility::Open
    } else if query_string.contains("current") {
        core::ContestVisibility::Current
    } else if query_string.contains("challenge") {
        core::ContestVisibility::LoginRequired
    } else {
        core::ContestVisibility::All
    };

    let config = req.get::<Read<SharedConfiguration>>().unwrap();

    let res = with_conn![core::show_contests, C, req, &session_token, login_info(&config), visibility];

    if res.is_err() {
        // Database connection failed … Create a new database connection!
        // TODO: This code should be unified with the database creation code in main.rs
        println!("DATABASE CONNECTION LOST! Restarting database connection.");
        let conn = C::reconnect(&config);
        let mutex = req.get::<Write<SharedDatabaseConnection<C>>>().unwrap();
        let mut sharedconn = mutex.lock().unwrap_or_else(|e| e.into_inner());
        *sharedconn = conn;
        // return ServerError();
    }

    let (template, mut data) = res.unwrap();

    data.insert("config".to_string(), to_json(&config.template_params));

    if query_string.contains("results") {
        data.insert("direct_link_to_results".to_string(), to_json(&true));
    }

    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}

fn contest<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let contest_id = req.expect_int::<i32>("contestid")?;
    let secret = req.get_str("secret");
    let session_token = req.require_session_token()?;
    let query_string = req.url.query().map(|s| s.to_string());

    let config = req.get::<Read<SharedConfiguration>>().unwrap();
    let (template, data) = with_conn![core::show_contest,
                                      C,
                                      req,
                                      contest_id,
                                      &session_token,
                                      query_string,
                                      login_info(&config),
                                      secret].aug(req)?;

    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}

fn contestresults<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let config = req.get::<Read<SharedConfiguration>>().unwrap();
    let disable_contest_results = config.disable_results_page.unwrap_or(false);

    if disable_contest_results {
        let mut resp = Response::new();
        resp.set_mut(Template::new(&"nocontestresults", 2)).set_mut(status::Locked);
        return Ok(resp);
    }

    let contest_id = req.expect_int::<i32>("contestid")?;
    let session_token = req.require_session_token()?;

    let (template, data) = with_conn![core::show_contest_results, C, req, contest_id, &session_token].aug(req)?;

    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}

fn contestresults_download<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let config = req.get::<Read<SharedConfiguration>>().unwrap();
    let disable_contest_results = config.disable_results_page.unwrap_or(false);

    if disable_contest_results {
        let mut resp = Response::new();
        resp.set_mut(Template::new(&"nocontestresults", 2)).set_mut(status::Locked);
        return Ok(resp);
    }

    let contest_id = req.expect_int::<i32>("contestid")?;
    let session_token = req.require_session_token()?;

    let (template, data) = with_conn![core::show_contest_results, C, req, contest_id, &session_token].aug(req)?;

    use iron::headers::{Charset, ContentDisposition, DispositionParam, DispositionType};

    let cd = ContentDisposition { disposition: DispositionType::Attachment,
                                  parameters: vec![DispositionParam::Filename(
        Charset::Ext("Utf-8".to_string()), // The character set for the bytes of the filename
        None,                              // The optional language tag (see `language-tag` crate)
        format!("{}.csv", data.get("contestname").unwrap().as_str().unwrap()).as_bytes().to_vec(), // the actual bytes of the filename
                                                                                                   // TODO: The name should be returned by core::show_contest_results directly
    )] };

    let mime: Mime = "text/csv".parse().unwrap();
    let mut resp = Response::new();
    resp.headers.set(cd);
    resp.set_mut(Template::new(&format!("{}_download", template), data)).set_mut(status::Ok).set_mut(mime);
    Ok(resp)
}

fn contest_post<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let contest_id = req.expect_int::<i32>("contestid")?;
    let session_token = req.expect_session_token()?;

    let (csrf_token, secret) = {
        let formdata = itry!(req.get_ref::<UrlEncodedBody>());
        (iexpect!(formdata.get("csrf_token"))[0].to_owned(), formdata.get("secret").map(|x| x[0].to_owned()))
    };

    // TODO: Was mit dem Result?
    with_conn![core::start_contest, C, req, contest_id, &session_token, &csrf_token, secret].aug(req)?;

    Ok(Response::with((status::Found, Redirect(url_for!(req, "contest", "contestid" => format!("{}",contest_id))))))
}

fn login<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let session_token = req.get_session_token();

    let config = req.get::<Read<SharedConfiguration>>().unwrap();
    let (template, mut data) = with_conn![core::show_login, C, req, session_token, login_info(&config)];

    let query_string = req.url.query().map(|s| s.to_string());
    if let Some(query) = query_string {
        data.insert("forward".to_string(), to_json(&query));
    }

    // Antwort erstellen und zurücksenden
    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}

fn login_post<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let logindata = {
        let formdata = itry!(req.get_ref::<UrlEncodedBody>());
        (iexpect!(formdata.get("username"))[0].to_owned(), iexpect!(formdata.get("password"))[0].to_owned())
    };

    let config = req.get::<Read<SharedConfiguration>>().unwrap();
    // TODO: Submit current session to login
    let loginresult = with_conn![core::login, C, req, logindata, login_info(&config)];

    match loginresult {
        // Login successful
        Ok(sessionkey) => {
            req.session().set(SessionToken { token: sessionkey }).unwrap();
            Ok(Response::with((status::Found, Redirect(url_for!(req, "greet")))))
        }
        // Login failed
        Err((template, data)) => {
            let mut resp = Response::new();
            resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
            Ok(resp)
        }
    }
}

fn login_code_post<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let code = {
        let formdata = itry!(req.get_ref::<UrlEncodedBody>());
        iexpect!(formdata.get("code"))[0].to_owned()
    };

    let config = req.get::<Read<SharedConfiguration>>().unwrap();
    // TODO: Submit current session to login
    let loginresult = with_conn![core::login_with_code, C, req, &code, login_info(&config)];

    match loginresult {
        // Login successful
        Ok(Ok(sessionkey)) => {
            req.session().set(SessionToken { token: sessionkey }).unwrap();
            Ok(Response::with((status::Found, Redirect(url_for!(req, "greet")))))
        }
        Ok(Err(sessionkey)) => {
            req.session().set(SessionToken { token: sessionkey }).unwrap();
            //Ok(Response::with((status::Found, Redirect(url_for!(req, "profile")))))
            Ok(Response::with((status::Found,
                               Redirect(iron::Url::parse(&format!("{}?status=firstlogin",
                                                                  &url_for!(req, "profile"))).unwrap()))))
        }
        // Login failed
        Err((template, data)) => {
            let mut resp = Response::new();
            resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
            Ok(resp)
        }
    }
}

fn logout<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let session_token = req.get_session_token();

    #[cfg(feature = "debug")]
    println!("Logging out session {:?}", session_token);

    with_conn![core::logout, C, req, session_token];

    Ok(Response::with((status::Found, Redirect(url_for!(req, "greet")))))
}

#[cfg(feature = "signup")]
fn signup<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let query_string = req.url.query().map(|s| s.to_string());

    let data = core::signupdata(query_string);
    let mut resp = Response::new();
    resp.set_mut(Template::new("signup", data)).set_mut(status::Ok);
    Ok(resp)
}

#[cfg(feature = "signup")]
fn signup_post<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let session_token = req.get_session_token();
    let signupdata = {
        let formdata = itry!(req.get_ref::<UrlEncodedBody>());
        (iexpect!(formdata.get("username"))[0].to_owned(),
         iexpect!(formdata.get("email"))[0].to_owned(),
         iexpect!(formdata.get("password"))[0].to_owned())
    };

    let signupresult = with_conn![core::signup, C, req, session_token, signupdata].aug(req)?;
    match signupresult {
        SignupResult::SignedUp => Ok(Response::with((status::Found,
                                                     Redirect(iron::Url::parse(&format!("{}?status={:?}",
                                                                                        &url_for!(req, "profile"),
                                                                                        signupresult)).unwrap())))),
        _ => Ok(Response::with((status::Found,
                                Redirect(iron::Url::parse(&format!("{}?status={:?}",
                                                                   &url_for!(req, "signup"),
                                                                   signupresult)).unwrap())))),
    }
}

#[cfg(not(feature = "signup"))]
fn signup<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    Err(core::MedalError::NotFound).aug(req).map_err(|x| x.into())
}

#[cfg(not(feature = "signup"))]
fn signup_post<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    Err(core::MedalError::NotFound).aug(req).map_err(|x| x.into())
}

fn submission<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let task_id = req.expect_int::<i32>("taskid")?;

    let session_token = req.expect_session_token()?;
    let subtask: Option<String> = (|| -> Option<String> {
        req.get_ref::<UrlEncodedQuery>().ok()?.get("subtask")?.get(0).map(|x| x.to_owned())
    })();

    let submission: Option<i32> = (|| -> Option<i32> {
        req.get_ref::<UrlEncodedQuery>().ok()?.get("submission")?.get(0).and_then(|x| x.parse::<i32>().ok())
    })();

    let result = with_conn![core::load_submission, C, req, task_id, &session_token, subtask, submission];

    match result {
        Ok(data) => Ok(Response::with((status::Ok, mime!(Application / Json), data))),
        Err(_) => Ok(Response::with((status::BadRequest, mime!(Application / Json), "{}".to_string()))),
    }
}

fn submission_post<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let task_id = req.expect_int::<i32>("taskid")?;
    let session_token = req.expect_session_token()?;
    let (csrf_token, data, grade, subtask) = {
        let formdata = iexpect!(req.get_ref::<UrlEncodedBody>().ok());
        (iexpect!(formdata.get("csrf_token"))[0].to_owned(),
         iexpect!(formdata.get("data"))[0].to_owned(),
         iexpect!(formdata.get("grade").unwrap_or(&vec!["0".to_owned()])[0].parse::<i32>().ok()),
         formdata.get("subtask").map(|x| x[0].to_owned()))
    };

    #[cfg(feature = "debug")]
    println!("New submission for task {} (graded {}): {}", task_id, grade, data);

    let result =
        with_conn![core::save_submission, C, req, task_id, &session_token, &csrf_token, data, grade, subtask].aug(req)?;

    Ok(Response::with((status::Ok, mime!(Application / Json), result)))
}

fn task<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let task_id = req.expect_int::<i32>("taskid")?;
    let session_token = req.require_session_token()?;

    // Get config value
    let autosaveinterval = {
        let config = req.get::<Read<SharedConfiguration>>().unwrap();
        config.auto_save_interval.unwrap_or(10)
    };

    match with_conn![core::show_task, C, req, task_id, &session_token, autosaveinterval].aug(req)? {
        Ok((template, data)) => {
            let mut resp = Response::new();
            resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
            Ok(resp)
        }
        Err(contest_id) => {
            // Idea: Append task, and if contest can be started immediately, we can just redirect again!
            Ok(Response::with((status::Found,
                               Redirect(url_for!(req, "contest", "contestid" => format!("{}",contest_id))))))
        }
    }
}

fn review<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let task_id = req.expect_int::<i32>("taskid")?;
    let submission_id = req.expect_int::<i32>("submissionid")?;
    let session_token = req.require_session_token()?;

    match with_conn![core::review_task, C, req, task_id, &session_token, submission_id].aug(req)? {
        Ok((template, data)) => {
            let mut resp = Response::new();
            resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
            Ok(resp)
        }
        Err(contest_id) => {
            // Idea: Append task, and if contest can be started immediately, we can just redirect again!
            Ok(Response::with((status::Found,
                               Redirect(url_for!(req, "contest", "contestid" => format!("{}",contest_id))))))
        }
    }
}

fn groups<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let session_token = req.require_session_token()?;

    let (template, data) = with_conn![core::show_groups, C, req, &session_token].aug(req)?;

    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}

fn group<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let group_id = req.expect_int::<i32>("groupid")?;
    let session_token = req.require_session_token()?;

    let (template, data) = with_conn![core::show_group, C, req, group_id, &session_token].aug(req)?;

    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}

fn group_download<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let group_id = req.expect_int::<i32>("groupid")?;
    let session_token = req.require_session_token()?;

    let (template, data) = with_conn![core::show_group, C, req, group_id, &session_token].aug(req)?;

    use iron::headers::{Charset, ContentDisposition, DispositionParam, DispositionType};

    let cd = ContentDisposition { disposition: DispositionType::Attachment,
                                  parameters: vec![DispositionParam::Filename(
        Charset::Ext("Utf-8".to_string()), // The character set for the bytes of the filename
        None,                              // The optional language tag (see `language-tag` crate)
        format!("{}.csv", data.get("groupname").unwrap().as_str().unwrap()).as_bytes().to_vec(), // the actual bytes of the filename
                                                                                                 // TODO: The name should be returned by core::show_group directly
    )] };

    let mime: Mime = "text/csv".parse().unwrap();
    let mut resp = Response::new();
    resp.headers.set(cd);
    resp.set_mut(Template::new(&format!("{}_download", template), data)).set_mut(status::Ok).set_mut(mime);
    Ok(resp)
}

//TODO: Secure with CSRF-Token?
fn group_post<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let group_id = req.expect_int::<i32>("groupid")?;
    let session_token = req.expect_session_token()?;

    //TODO: use result?
    with_conn![core::modify_group, C, req, group_id, &session_token].aug(req)?;

    Ok(Response::with((status::Found, Redirect(url_for!(req, "group", "groupid" => format!("{}",group_id))))))
}

fn new_group<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let session_token = req.require_session_token()?;

    let (csrf_token, name, tag) = {
        let formdata = iexpect!(req.get_ref::<UrlEncodedBody>().ok());
        (iexpect!(formdata.get("csrf_token"))[0].to_owned(),
         iexpect!(formdata.get("name"))[0].to_owned(),
         iexpect!(formdata.get("tag"))[0].to_owned())
    };

    let group_id = with_conn![core::add_group, C, req, &session_token, &csrf_token, name, tag].aug(req)?;

    Ok(Response::with((status::Found, Redirect(url_for!(req, "group", "groupid" => format!("{}",group_id))))))
}

fn group_csv<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let session_token = req.require_session_token()?;

    let si = {
        let config = req.get::<Read<SharedConfiguration>>().unwrap();
        core::SexInformation { require_sex: config.require_sex.unwrap_or(false),
                               allow_sex_na: config.allow_sex_na.unwrap_or(true),
                               allow_sex_diverse: config.allow_sex_diverse.unwrap_or(false),
                               allow_sex_other: config.allow_sex_other.unwrap_or(true) }
    };

    template_ok!(with_conn![core::group_csv, C, req, &session_token, si].aug(req)?)
}

fn group_csv_upload<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let session_token = req.require_session_token()?;

    let (csrf_token, group_data) = {
        let formdata = iexpect!(req.get_ref::<UrlEncodedBody>().ok());
        (iexpect!(formdata.get("csrf_token"))[0].to_owned(), iexpect!(formdata.get("group_data"))[0].to_owned())
    };

    #[cfg(feature = "debug")]
    println!("{}", group_data);

    with_conn![core::upload_groups, C, req, &session_token, &csrf_token, &group_data].aug(req)?;

    Ok(Response::with((status::Found, Redirect(url_for!(req, "groups")))))
}

fn profile<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let session_token = req.require_session_token()?;
    let query_string = req.url.query().map(|s| s.to_string());

    let si = {
        let config = req.get::<Read<SharedConfiguration>>().unwrap();
        core::SexInformation { require_sex: config.require_sex.unwrap_or(false),
                               allow_sex_na: config.allow_sex_na.unwrap_or(true),
                               allow_sex_diverse: config.allow_sex_diverse.unwrap_or(false),
                               allow_sex_other: config.allow_sex_other.unwrap_or(true) }
    };

    let (template, data) = with_conn![core::show_profile, C, req, &session_token, None, query_string, si].aug(req)?;

    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}

fn profile_post<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let session_token = req.expect_session_token()?;
    let (csrf_token, firstname, lastname, street, zip, city, pwd, pwd_repeat, grade, sex) = {
        let formdata = itry!(req.get_ref::<UrlEncodedBody>());
        (iexpect!(formdata.get("csrf_token"))[0].to_owned(),
         iexpect!(formdata.get("firstname"))[0].to_owned(),
         iexpect!(formdata.get("lastname"))[0].to_owned(),
         formdata.get("street").map(|x| x[0].to_owned()),
         formdata.get("zip").map(|x| x[0].to_owned()),
         formdata.get("city").map(|x| x[0].to_owned()),
         formdata.get("password").map(|x| x[0].to_owned()),
         formdata.get("password_repeat").map(|x| x[0].to_owned()),
         iexpect!(formdata.get("grade"))[0].parse::<i32>().unwrap_or(0),
         iexpect!(formdata.get("sex"))[0].parse::<i32>().ok())
    };

    let profilechangeresult =
        with_conn![core::edit_profile,
                   C,
                   req,
                   &session_token,
                   None,
                   &csrf_token,
                   (firstname, lastname, street, zip, city, pwd, pwd_repeat, grade, sex)].aug(req)?;

    Ok(Response::with((status::Found,
                       Redirect(iron::Url::parse(&format!("{}?status={:?}",
                                                          &url_for!(req, "profile"),
                                                          profilechangeresult)).unwrap()))))
}

fn user<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let user_id = req.expect_int::<i32>("userid")?;
    let session_token = req.expect_session_token()?;
    let query_string = req.url.query().map(|s| s.to_string());

    let si = {
        let config = req.get::<Read<SharedConfiguration>>().unwrap();
        core::SexInformation { require_sex: config.require_sex.unwrap_or(false),
                               allow_sex_na: config.allow_sex_na.unwrap_or(true),
                               allow_sex_diverse: config.allow_sex_diverse.unwrap_or(false),
                               allow_sex_other: config.allow_sex_other.unwrap_or(true) }
    };

    let (template, data) =
        with_conn![core::show_profile, C, req, &session_token, Some(user_id), query_string, si].aug(req)?;

    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}

fn user_post<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let user_id = req.expect_int::<i32>("userid")?;
    let session_token = req.expect_session_token()?;
    let (csrf_token, firstname, lastname, street, zip, city, pwd, pwd_repeat, grade, sex) = {
        let formdata = itry!(req.get_ref::<UrlEncodedBody>());
        (iexpect!(formdata.get("csrf_token"))[0].to_owned(),
         iexpect!(formdata.get("firstname"))[0].to_owned(),
         iexpect!(formdata.get("lastname"))[0].to_owned(),
         formdata.get("street").map(|x| x[0].to_owned()),
         formdata.get("zip").map(|x| x[0].to_owned()),
         formdata.get("city").map(|x| x[0].to_owned()),
         formdata.get("password").map(|x| x[0].to_owned()),
         formdata.get("password_repeat").map(|x| x[0].to_owned()),
         iexpect!(formdata.get("grade"))[0].parse::<i32>().unwrap_or(0),
         iexpect!(formdata.get("sex"))[0].parse::<i32>().ok())
    };

    let profilechangeresult =
        with_conn![core::edit_profile,
                   C,
                   req,
                   &session_token,
                   Some(user_id),
                   &csrf_token,
                   (firstname, lastname, street, zip, city, pwd, pwd_repeat, grade, sex)].aug(req)?;

    Ok(Response::with((status::Found,
                       Redirect(iron::Url::parse(&format!("{}?status={:?}",
                                                          &url_for!(req, "user", "userid" => format!("{}",user_id)),
                                                          profilechangeresult)).unwrap()))))
    //old:   Ok(Response::with((status::Found, Redirect(url_for!(req, "user", "userid" => format!("{}",user_id))))))
}

fn teacherinfos<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let session_token = req.expect_session_token()?;

    let config = req.get::<Read<SharedConfiguration>>().unwrap();

    let (template, mut data) = with_conn![core::teacher_infos, C, req, &session_token].aug(req)?;

    data.insert("config".to_string(), to_json(&config.template_params));

    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}

fn admin<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let session_token = req.expect_session_token()?;

    let config = req.get::<Read<SharedConfiguration>>().unwrap();

    let (template, mut data) = with_conn![core::admin_index, C, req, &session_token].aug(req)?;

    data.insert("dbstatus_secret".to_string(), to_json(&config.dbstatus_secret));

    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}

fn admin_users<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let session_token = req.expect_session_token()?;

    let (s_id, s_firstname, s_lastname, s_logincode, s_groupcode, s_pms_id) = {
        let formdata = itry!(req.get_ref::<UrlEncodedBody>());
        (formdata.get("id").map(|x| x[0].parse::<i32>().unwrap_or(0)),
         formdata.get("firstname").map(|x| x[0].to_owned()),
         formdata.get("lastname").map(|x| x[0].to_owned()),
         formdata.get("logincode").map(|x| x[0].to_owned()),
         formdata.get("groupcode").map(|x| x[0].to_owned()),
         formdata.get("pmsid").map(|x| x[0].to_owned()))
    };

    let (template, data) = with_conn![core::admin_search_users,
                                      C,
                                      req,
                                      &session_token,
                                      (s_id, s_firstname, s_lastname, s_logincode, s_groupcode, s_pms_id)].aug(req)?;

    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}

fn admin_user<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let user_id = req.expect_int::<i32>("userid")?;
    let session_token = req.expect_session_token()?;

    let (csrf_token, group_id) = if let Ok(formdata) = req.get_ref::<UrlEncodedBody>() {
        // or iexpect!(formdata.get("csrf_token"))[0].to_owned(), ?
        (formdata.get("csrf_token").map(|x| x[0].to_owned()),
         formdata.get("group_id").map(|x| x[0].parse::<i32>().unwrap_or(0)))
    } else {
        (None, None)
    };

    let (template, data) = if let Some(csrf_token) = csrf_token {
        if let Some(group_id) = group_id {
            with_conn![core::admin_move_user_to_group, C, req, user_id, group_id, &session_token, &csrf_token].aug(req)?
        } else {
            with_conn![core::admin_delete_user, C, req, user_id, &session_token, &csrf_token].aug(req)?
        }
    } else {
        with_conn![core::admin_show_user, C, req, user_id, &session_token].aug(req)?
    };

    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}

fn admin_group<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let group_id = req.expect_int::<i32>("groupid")?;
    let session_token = req.expect_session_token()?;

    let csrf_token = if let Ok(formdata) = req.get_ref::<UrlEncodedBody>() {
        formdata.get("csrf_token").map(|x| x[0].to_owned())
    } else {
        None
    };

    let (template, data) = if let Some(csrf_token) = csrf_token {
        with_conn![core::admin_delete_group, C, req, group_id, &session_token, &csrf_token].aug(req)?
    } else {
        with_conn![core::admin_show_group, C, req, group_id, &session_token].aug(req)?
    };

    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}

fn admin_participation<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let user_id = req.expect_int::<i32>("userid")?;
    let contest_id = req.expect_int::<i32>("contestid")?;
    let session_token = req.expect_session_token()?;

    let csrf_token = if let Ok(formdata) = req.get_ref::<UrlEncodedBody>() {
        formdata.get("csrf_token").map(|x| x[0].to_owned())
    } else {
        None
    };

    let (template, data) = if let Some(csrf_token) = csrf_token {
        with_conn![core::admin_delete_participation, C, req, user_id, contest_id, &session_token, &csrf_token].aug(req)?
    } else {
        with_conn![core::admin_show_participation, C, req, user_id, contest_id, &session_token].aug(req)?
    };

    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}

fn admin_contests<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let session_token = req.expect_session_token()?;

    let (template, data) = with_conn![core::admin_show_contests, C, req, &session_token].aug(req)?;

    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}

fn admin_export_contest<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let contest_id = req.expect_int::<i32>("contestid")?;
    let session_token = req.expect_session_token()?;

    let filename = with_conn![core::admin_contest_export, C, req, contest_id, &session_token].aug(req)?;

    Ok(Response::with((status::Found, RedirectRaw(format!("/export/{}", filename)))))
}

fn admin_cleanup<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let session_token = req.expect_session_token()?;

    let csrf_token = if let Ok(formdata) = req.get_ref::<UrlEncodedBody>() {
        formdata.get("csrf_token").map(|x| x[0].to_owned())
    } else {
        None
    };

    let (template, data) = if let Some(csrf_token) = csrf_token {
        let cleanup_type = req.get_str("type");

        match cleanup_type.as_deref() {
            Some("session") => with_conn![core::do_session_cleanup, C, req,].aug(req)?,
            _ => with_conn![core::admin_do_cleanup, C, req, &session_token, &csrf_token].aug(req)?,
        }
    } else {
        with_conn![core::admin_show_cleanup, C, req, &session_token].aug(req)?
    };

    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}

fn dbcleanup<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let (template, data) = with_conn![core::do_session_cleanup, C, req,].aug(req)?;

    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}

fn oauth<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    #[cfg(feature = "debug")]
    println!("{:?}", req.url.query().unwrap_or(""));

    let oauth_id = req.expect_str("oauthid")?;
    let school_id = req.get_str("schoolid");

    let oauth_provider = {
        let config = req.get::<Read<SharedConfiguration>>().unwrap();

        let mut result: Option<OauthProvider> = None;

        if let Some(ref oauth_providers) = config.oauth_providers {
            for oauth_provider in oauth_providers {
                if oauth_provider.provider_id == oauth_id {
                    result = Some(oauth_provider.clone());
                    break;
                }
            }

            if let Some(result) = result {
                result
            } else {
                return Ok(Response::with(iron::status::NotFound));
            }
        } else {
            return Ok(Response::with(iron::status::NotFound));
        }
    };

    let user_data_result = match oauth_provider.medal_oauth_type.as_ref() {
        "pms" => oauth_pms(req, oauth_provider, school_id.as_ref()).aug(req)?,
        _ => return Ok(Response::with(iron::status::NotFound)),
    };

    let user_data = match user_data_result {
        Err(response) => return Ok(response),
        Ok(user_data) => user_data,
    };
    let user_type = user_data.foreign_type;

    let oauthloginresult = {
        // hier ggf. Daten aus dem Request holen
        let mutex = req.get::<Write<SharedDatabaseConnection<C>>>().unwrap();
        let conn = mutex.lock().unwrap_or_else(|e| e.into_inner());

        // Antwort erstellen und zurücksenden
        core::login_oauth(&*conn, user_data, oauth_id)
    };

    match oauthloginresult {
        // Login successful
        Ok((sessionkey, redirectprofile)) => {
            req.session().set(SessionToken { token: sessionkey }).unwrap();

            use core::UserType;
            if user_type == UserType::User && redirectprofile {
                Ok(Response::with((status::Found,
                                   Redirect(iron::Url::parse(&format!("{}?status=firstlogin",
                                                                      &url_for!(req, "profile"))).unwrap()))))
            } else {
                Ok(Response::with((status::Found, Redirect(url_for!(req, "greet")))))
            }
        }
        // Login failed
        Err((template, data)) => {
            let mut resp = Response::new();
            resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
            Ok(resp)
        }
    }
}

#[derive(Deserialize, Debug)]
struct OAuthAccess {
    access_token: String,
    #[allow(dead_code)]
    token_type: String,
    #[allow(dead_code)]
    refresh_token: String,
    #[allow(dead_code)]
    expires: Option<i32>, // documented as 'expires_in'
    #[allow(dead_code)]
    expires_in: Option<i32>, // sent as 'expires'
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
#[serde(untagged)]
pub enum SchoolIdOrSchoolIds {
    None(i32), // PMS sends -1 here if user has no schools associated (admin, jury)
    SchoolId(String),
    SchoolIds(Vec<String>),
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct OAuthUserData {
    userID: Option<String>, // documented as 'userId'
    userId: Option<String>, // sent as 'userID'
    userType: String,
    gender: String,
    firstName: String,
    lastName: String,
    #[allow(dead_code)]
    dateOfBirth: Option<String>,
    #[allow(dead_code)]
    eMail: Option<String>,
    schoolId: Option<SchoolIdOrSchoolIds>,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct OAuthSchoolData {
    name: Option<String>,
    city: Option<String>,
    error: Option<String>,
}

fn pms_hash_school(school_id: &str, secret: &str) -> String {
    use sha2::{Digest, Sha512};
    let mut hasher = Sha512::default();
    let string_to_hash = format!("{}{}", school_id, secret);

    hasher.input(string_to_hash.as_bytes());
    let hashed_string = hasher.result();

    format!("{:02X?}", hashed_string).chars().filter(|c| c.is_ascii_alphanumeric()).collect()
}

fn oauth_pms(req: &mut Request, oauth_provider: OauthProvider, selected_school_id: Option<&String>)
             -> Result<Result<core::ForeignUserData, Response>, core::MedalError> {
    use core::{UserSex, UserType};
    use params::{Params, Value};

    fn er(e: &str) -> core::MedalError { core::MedalError::OauthError(e.to_string()) }
    fn e<T>(e: &str) -> Result<T, core::MedalError> { Err::<T, _>(er(e)) }

    let (_, _, code): (String, String, String) = {
        let map = req.get_ref::<Params>().unwrap();

        match (map.find(&["state"]), map.find(&["scope"]), map.find(&["code"])) {
            (Some(&Value::String(ref state)), Some(&Value::String(ref scope)), Some(&Value::String(ref code)))
                if state == "42" =>
            {
                (state.clone(), scope.clone(), code.clone())
            }
            _ => return e("#70"),
        }
    };

    let client = reqwest::Client::new();
    let params = [("code", code), ("grant_type", "authorization_code".to_string())];

    // TODO: This can fail if to much time has passed
    let res = client.post(&oauth_provider.access_token_url)
                    .basic_auth(oauth_provider.client_id, Some(oauth_provider.client_secret))
                    .form(&params)
                    .send();
    let access: OAuthAccess = res.or(e("#00"))?.json().or(e("#01"))?;

    let res = client.get(&oauth_provider.user_data_url).bearer_auth(access.access_token).send();
    let mut user_data: OAuthUserData = res.or(e("#10"))?.json().or(e("#11"))?;

    // Unify ambiguous fields
    user_data.userId = user_data.userID.or(user_data.userId);

    let user_type = match user_data.userType.as_ref() {
        "a" | "A" => UserType::Admin,
        "t" | "T" => UserType::Teacher,
        "s" | "S" => UserType::User,
        _ => UserType::User,
    };
    let user_sex = match user_data.gender.as_ref() {
        "m" | "M" => UserSex::Male,
        "f" | "F" | "w" | "W" => UserSex::Female,
        "?" => UserSex::Unknown,
        _ => UserSex::Unknown,
    };

    match (&user_data.schoolId, user_type) {
        // Students cannot have a list of schools
        (Some(SchoolIdOrSchoolIds::SchoolIds(_)), UserType::User) => return e("#70"),
        // If we need to make sure, a student has a school, we should add the case None and Some(None(_)) here

        // Teachers must have a list of schools
        (Some(SchoolIdOrSchoolIds::SchoolId(_)), UserType::Teacher) => return e("#71"),
        (Some(SchoolIdOrSchoolIds::None(_)), UserType::Teacher) => return e("#72"),
        // Convert no schools to empty list
        (None, UserType::Teacher) => {
            user_data.schoolId = Some(SchoolIdOrSchoolIds::SchoolIds(Vec::new()));
        }

        // For other users, we currently don't care
        _ => (),
    }

    // Does the user has an array of school (i.e. is he a teacher)?
    if let Some(SchoolIdOrSchoolIds::SchoolIds(school_ids)) = user_data.schoolId {
        // Has there been a school selected?
        if let Some(selected_school_id) = selected_school_id {
            if selected_school_id == "none" && oauth_provider.allow_teacher_login_without_school == Some(true) {
                // Nothing to do
            }
            // Is the school a valid school for the user?
            else if school_ids.contains(&selected_school_id) {
                if let Some(mut user_id) = user_data.userId {
                    user_id.push('/');
                    user_id.push_str(&selected_school_id);
                    user_data.userId = Some(user_id);
                }
            } else {
                return e("#40");
            }
        } else {
            // No school has been selected
            // Check if school data query is configured. Otherwise there is nothing to do.
            if let (Some(school_data_url), Some(school_data_secret)) =
                (oauth_provider.school_data_url, oauth_provider.school_data_secret)
            {
                // Gather school information of all schools
                let school_infos: Vec<(String, String)> =
                    school_ids.iter()
                              .map(|school_id| -> Result<(String, String), core::MedalError> {
                                  let params = [("schoolId", school_id.clone()),
                                                ("hash", pms_hash_school(&school_id, &school_data_secret))];
                                  let res = client.post(&school_data_url).form(&params).send();
                                  let school_data: OAuthSchoolData = res.or(e("#30"))?.json().or(e("#31"))?;

                                  Ok((school_id.clone(),
                                      format!("{}, {}",
                                              school_data.name
                                                         .or(school_data.error)
                                                         .unwrap_or_else(|| "Information missing".to_string()),
                                              school_data.city.unwrap_or_else(|| "–".to_string()))))
                              })
                              .collect::<Result<_, _>>()?;

                let mut data = json_val::Map::new();
                data.insert("schools".to_string(), to_json(&school_infos));
                data.insert("query".to_string(), to_json(&req.url.query().unwrap_or("")));

                data.insert("parent".to_string(), to_json(&"base"));
                data.insert("disable_login_box".to_string(), to_json(&true));

                data.insert("teacher_login_without_school".to_string(),
                            to_json(&oauth_provider.allow_teacher_login_without_school.unwrap_or(false)));

                let mut resp = Response::new();
                resp.set_mut(Template::new(&"oauth_school_selector", data)).set_mut(status::Ok);
                return Ok(Err(resp));
            } else {
                // Configuration error:
                return Err(core::MedalError::ConfigurationError);
            }
        }
    } else if selected_school_id.is_some() {
        // A school has apparently been selected but the user is actually not a teacher
        return e("#50");
    }

    Ok(Ok(core::ForeignUserData { foreign_id: user_data.userId.ok_or(er("#60"))?,
                                  foreign_type: user_type,
                                  sex: user_sex,
                                  firstname: user_data.firstName,
                                  lastname: user_data.lastName }))
}

// Share Database connection between workers
#[derive(Copy, Clone)]
pub struct SharedDatabaseConnection<C>
    where C: MedalConnection
{
    phantom: std::marker::PhantomData<C>,
}
impl<C> Key for SharedDatabaseConnection<C> where C: MedalConnection + 'static
{
    type Value = C;
}

// Share Configuration between workers
#[derive(Copy, Clone)]
pub struct SharedConfiguration;
impl Key for SharedConfiguration {
    type Value = Config;
}

#[cfg(feature = "watch")]
pub fn get_handlebars_engine(template_name: &str) -> impl AfterMiddleware {
    // HandlebarsEngine will look up all files with "./examples/templates/**/*.hbs"
    let mut hbse = HandlebarsEngine::new();
    hbse.add(Box::new(DirectorySource::new(&format!("./templates/{}/", template_name) as &str, ".hbs")));

    // load templates from all registered sources
    if let Err(r) = hbse.reload() {
        panic!("{}", r);
    }

    use handlebars_iron::Watchable;
    use std::sync::Arc;

    let hbse_ref = Arc::new(hbse);
    hbse_ref.watch("./templates/");
    hbse_ref
}

#[cfg(not(feature = "watch"))]
pub fn get_handlebars_engine(template_name: &str) -> impl AfterMiddleware {
    // HandlebarsEngine will look up all files with "./templates/<template>/**/*.hbs"
    let mut hbse = HandlebarsEngine::new();
    hbse.add(Box::new(DirectorySource::new(&format!("./templates/{}/", template_name) as &str, ".hbs")));

    // load templates from all registered sources
    if let Err(r) = hbse.reload() {
        panic!("{}", r);
    }

    hbse
}

fn cookie_warning(req: &mut Request) -> IronResult<Response> {
    match req.get_session_token() {
        Some(_session_token) => {
            // TODO: Set session!
            // TODO:
            Ok(Response::with((status::Found, RedirectRaw(format!("/{}", req.url.query().unwrap_or(""))))))
        }
        None => {
            let mut resp = Response::new();
            resp.set_mut(Template::new("cookie", json_val::Map::new())).set_mut(status::Ok);
            Ok(resp)
        }
    }
}

pub fn start_server<C>(conn: C, config: Config) -> iron::error::HttpResult<iron::Listening>
    where C: MedalConnection + std::marker::Send + 'static {
    let router = router!(
        greet: get "/" => greet_personal::<C>,
        contests: get "/contest/" => contests::<C>,
        contest: get "/contest/:contestid" => contest::<C>,
        contest_secret: get "/contest/:contestid/:secret" => contest::<C>,
        contestresults: get "/contest/:contestid/result/" => contestresults::<C>,
        contestresults_download: get "/contest/:contestid/result/download" => contestresults_download::<C>,
        contest_post: post "/contest/:contestid" => contest_post::<C>,
        contest_post_secret: post "/contest/:contestid/:secret" => contest_post::<C>, // just ignoring the secret
        login: get "/login" => login::<C>,
        login_post: post "/login" => login_post::<C>,
        login_code_post: post "/clogin" => login_code_post::<C>,
        logout: get "/logout" => logout::<C>,
        signup: get "/signup" => signup::<C>,
        signup_post: post "/signup" => signup_post::<C>,
        subm_load: get "/load/:taskid" => submission::<C>,
        subm_save: post "/save/:taskid" => submission_post::<C>,
        groups: get "/group/" => groups::<C>,
        groups: post "/group/" => new_group::<C>,
        group: get "/group/:groupid" => group::<C>,
        group_download: get "/group/download/:groupid" => group_download::<C>,
        group_post: post "/group" => group_post::<C>,
        groupcsv: get "/group/csv" => group_csv::<C>,
        groupcsv_post: post "/group/csv" => group_csv_upload::<C>,
        profile: get "/profile" => profile::<C>,
        profile_post: post "/profile" => profile_post::<C>,
        user: get "/user/:userid" => user::<C>,
        user_post: post "/user/:userid" => user_post::<C>,
        task: get "/task/:taskid" => task::<C>,
        task_review_solution: get "/task/:taskid/:submissionid" => review::<C>,
        teacher: get "/teacher" => teacherinfos::<C>,
        admin: get "/admin" => admin::<C>,
        admin_users: post "/admin/user/" => admin_users::<C>,
        admin_user: get "/admin/user/:userid" => admin_user::<C>,
        admin_user_post: post "/admin/user/:userid" => admin_user::<C>,
        admin_group: get "/admin/group/:groupid" => admin_group::<C>,
        admin_group_post: post "/admin/group/:groupid" => admin_group::<C>,
        admin_participation: get "/admin/user/:userid/:contestid" => admin_participation::<C>,
        admin_participation_post: post "/admin/user/:userid/:contestid" => admin_participation::<C>,
        admin_contests: get "/admin/contest/" => admin_contests::<C>,
        admin_export_contest: get "/admin/contest/:contestid/export" => admin_export_contest::<C>,
        admin_cleanup: get "/admin/cleanup" => admin_cleanup::<C>,
        admin_cleanup_post: post "/admin/cleanup/:type" => admin_cleanup::<C>,
        oauth: get "/oauth/:oauthid/" => oauth::<C>,
        oauth_school: get "/oauth/:oauthid/:schoolid" => oauth::<C>,
        check_cookie: get "/cookie" => cookie_warning,
        dbstatus: get "/dbstatus" => dbstatus::<C>,
        status: get "/status" => dbstatus::<C>,
        dbcleanup: get "/cleanup" => dbcleanup::<C>,
        debug: get "/debug" => debug::<C>,
        debug_reset: get "/debug/reset" => debug_new_token::<C>,
        debug_logout: get "/debug/logout" => debug_logout::<C>,
        debug_create: get "/debug/create" => debug_create_session::<C>,
    );

    let mut mount = Mount::new();

    // Serve the shared JS/CSS at /
    mount.mount("/static/", Static::new(Path::new("static")));
    mount.mount("/export/", Static::new(Path::new("export")));
    mount.mount("/tasks/", Static::new(Path::new(TASK_DIR)));
    mount.mount("/", router);

    let mut ch = Chain::new(mount);

    #[cfg(feature = "debug")]
    ch.link_before(RequestLogger {});

    ch.link(Write::<SharedDatabaseConnection<C>>::both(conn));
    ch.link(Read::<SharedConfiguration>::both(config.clone()));

    ch.link_around(RequestTimeLogger {});
    ch.link_around(CookieDistributor {});
    ch.link_around(SessionStorage::new(SignedCookieBackend::new(config.cookie_signing_secret.expect("Cookie signing secret not found in configuration").into_bytes())));

    ch.link_after(get_handlebars_engine(&config.template.unwrap_or_else(|| "default".to_string())));
    ch.link_after(ErrorReporter);
    ch.link_after(ErrorShower);

    let socket_addr = format!("{}:{}", config.host.unwrap(), config.port.unwrap());

    let srvr = Iron::new(ch).http(&socket_addr);
    print!("Listening on {} … ", &socket_addr);
    srvr
}
