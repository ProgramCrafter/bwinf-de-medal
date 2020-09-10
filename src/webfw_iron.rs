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

use std::path::Path;

pub use handlebars_iron::handlebars::to_json;
use handlebars_iron::{DirectorySource, HandlebarsEngine, Template};
use iron;
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

use config::Config;
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

/** Show error messages on commandline */
struct ErrorReporter;
impl AfterMiddleware for ErrorReporter {
    fn catch(&self, _: &mut Request, err: IronError) -> IronResult<Response> {
        println!("{}", err);
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
                    if n >= 400 && n <= 599 {
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
    fn get_session_token(self: &mut Self) -> Option<String> {
        let session_token = self.session().get::<SessionToken>().unwrap();
        (|st: Option<SessionToken>| -> Option<String> { Some(st?.token) })(session_token)
    }

    fn require_session_token(self: &mut Self) -> IronResult<String> {
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

    fn expect_session_token(self: &mut Self) -> IronResult<String> {
        match self.session().get::<SessionToken>().unwrap() {
            Some(SessionToken { token: session }) => Ok(session),
            _ => Err(IronError { error: Box::new(SessionError { message:
                                                                    "No valid session found, access denied".to_string() }),
                                 response: Response::with(status::Forbidden) }),
        }
    }
}

trait RequestRouterParam {
    fn get_str(self: &mut Self, key: &str) -> Option<String>;
    fn get_int<T: ::std::str::FromStr>(self: &mut Self, key: &str) -> Option<T>;
    fn expect_int<T: ::std::str::FromStr>(self: &mut Self, key: &str) -> IronResult<T>;
    fn expect_str(self: &mut Self, key: &str) -> IronResult<String>;
}

impl<'a, 'b> RequestRouterParam for Request<'a, 'b> {
    fn get_str(self: &mut Self, key: &str) -> Option<String> {
        Some(self.extensions.get::<Router>()?.find(key)?.to_owned())
    }

    fn get_int<T: ::std::str::FromStr>(self: &mut Self, key: &str) -> Option<T> {
        Some(self.extensions.get::<Router>()?.find(key)?.parse::<T>().ok()?)
    }

    fn expect_int<T: ::std::str::FromStr>(self: &mut Self, key: &str) -> IronResult<T> {
        match self.get_int::<T>(key) {
            Some(i) => Ok(i),
            _ => Err(IronError { error: Box::new(SessionError { message:
                                                                    "No valid routing parameter".to_string() }),
                                 response: Response::with(status::Forbidden) }),
        }
    }

    fn expect_str(self: &mut Self, key: &str) -> IronResult<String> {
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
            core::MedalError::PasswordHashingError => {
                IronError { error: Box::new(SessionError { message: "Error hashing the passwords".to_string() }),
                            response: Response::with(status::InternalServerError) }
            }
            core::MedalError::UnmatchedPasswords => {
                IronError { error: Box::new(SessionError { message:
                                                               "The two passwords did not match.".to_string() }),
                            response: Response::with(status::Forbidden) }
            }
            core::MedalError::NotFound => {
                IronError { error: Box::new(SessionError { message: "Not found".to_string() }),
                            response: Response::with(status::NotFound) }
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
    let (template, mut data) = with_conn![core::index, C, req, session_token, login_info(&config)];

    /*if let Some(server_message) = &config.server_message {
        data.insert("server_message".to_string(), to_json(&server_message));
    }*/
    config.server_message.as_ref().map(|sm| data.insert("server_message".to_string(), to_json(&sm)));

    // Antwort erstellen und zurücksenden
    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}

fn dbstatus<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let status = with_conn![core::status, C, req, ()];

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
    let (template, mut data) = with_conn![core::show_contests, C, req, &session_token, login_info(&config), visibility];

    config.server_message.as_ref().map(|sm| data.insert("server_message".to_string(), to_json(&sm)));

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
    let session_token = req.require_session_token()?;
    let query_string = req.url.query().map(|s| s.to_string());

    let config = req.get::<Read<SharedConfiguration>>().unwrap();
    let (template, data) =
        with_conn![core::show_contest, C, req, contest_id, &session_token, query_string, login_info(&config)].aug(req)?;

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

    println!("test");

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

    let mut resp = Response::new();
    resp.headers.set(cd);
    resp.set_mut(Template::new(&format!("{}_download", template), data)).set_mut(status::Ok);
    Ok(resp)
}

fn contest_post<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let contest_id = req.expect_int::<i32>("contestid")?;
    let session_token = req.expect_session_token()?;

    let csrf_token = {
        let formdata = itry!(req.get_ref::<UrlEncodedBody>());
        iexpect!(formdata.get("csrf_token"))[0].to_owned()
    };

    // TODO: Was mit dem Result?
    with_conn![core::start_contest, C, req, contest_id, &session_token, &csrf_token].aug(req)?;

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

    println!("Loggin out session {:?}", session_token);

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
        (iexpect!(formdata.get("username"))[0].to_owned(), iexpect!(formdata.get("email"))[0].to_owned(), iexpect!(formdata.get("password"))[0].to_owned())
    };

    let signupresult = with_conn![core::signup, C, req, session_token, signupdata].aug(req)?;
    match signupresult {
        SignupResult::SignedUp =>
            Ok(Response::with((status::Found,
                       Redirect(iron::Url::parse(&format!("{}?status={:?}",
                                                          &url_for!(req, "profile"),
                                                          signupresult)).unwrap())))),
        _ =>
            Ok(Response::with((status::Found,
                       Redirect(iron::Url::parse(&format!("{}?status={:?}",
                                                          &url_for!(req, "signup"),
                                                          signupresult)).unwrap()))))
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

    let result = with_conn![core::load_submission, C, req, task_id, &session_token, subtask];

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

    let (template, data) = with_conn![core::show_task, C, req, task_id, &session_token].aug(req)?;

    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
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

    let mut resp = Response::new();
    resp.headers.set(cd);
    resp.set_mut(Template::new(&format!("{}_download", template), data)).set_mut(status::Ok);
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

    template_ok!(with_conn![core::group_csv, C, req, &session_token].aug(req)?)
}

fn group_csv_upload<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let session_token = req.require_session_token()?;

    let (csrf_token, group_data) = {
        let formdata = iexpect!(req.get_ref::<UrlEncodedBody>().ok());
        (iexpect!(formdata.get("csrf_token"))[0].to_owned(), iexpect!(formdata.get("group_data"))[0].to_owned())
    };

    println!("{}", group_data);

    with_conn![core::upload_groups, C, req, &session_token, &csrf_token, &group_data].aug(req)?;

    Ok(Response::with((status::Found, Redirect(url_for!(req, "groups")))))
}

fn profile<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let session_token = req.require_session_token()?;
    let query_string = req.url.query().map(|s| s.to_string());

    let (template, data) = with_conn![core::show_profile, C, req, &session_token, None, query_string].aug(req)?;

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

    let (template, data) =
        with_conn![core::show_profile, C, req, &session_token, Some(user_id), query_string].aug(req)?;

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

    let (template, data) =
        with_conn![core::teacher_infos, C, req, &session_token, config.teacher_page.as_ref().map(|x| &**x)].aug(req)?;
    // .as_ref().map(|x| &**x) can be written as .as_deref() since rust 1.40

    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}

fn admin<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    let session_token = req.expect_session_token()?;

    let (template, data) = with_conn![core::admin_index, C, req, &session_token].aug(req)?;

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

    let csrf_token = if let Ok(formdata) = req.get_ref::<UrlEncodedBody>() {
        // or iexpect!(formdata.get("csrf_token"))[0].to_owned(), ?
        formdata.get("csrf_token").map(|x| x[0].to_owned())
    } else {
        None
    };

    let (template, data) = if let Some(csrf_token) = csrf_token {
        with_conn![core::admin_delete_user, C, req, user_id, &session_token, &csrf_token].aug(req)?
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

#[derive(Deserialize, Debug)]
struct OAuthAccess {
    access_token: String,
    token_type: String,
    refresh_token: String,
    expires: Option<i32>,    // documented as 'expires_in'
    expires_in: Option<i32>, // sent as 'expires'
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
#[serde(untagged)]
pub enum SchoolIdOrSchoolIds {
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
    dateOfBirth: Option<String>,
    eMail: Option<String>,
    schoolId: Option<SchoolIdOrSchoolIds>,
    userId_int: Option<String>,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct OAuthSchoolTeacherData {
    lastName: String,
    gender: String,
    firstName: String,
    email: String,
    teacherId: String,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct OAuthSchoolData {
    contact: Option<String>,
    zip: Option<String>,
    name: Option<String>,
    federalStateID: Option<String>,
    teachers: Option<Vec<OAuthSchoolTeacherData>>,
    city: Option<String>,
    schoolType: Option<String>,
    url: Option<String>,
    street1: Option<String>,
    schoolId: Option<String>,
    error: Option<String>,
}

// TODO: Most of this code should be moved into core:: as a new function
fn oauth<C>(req: &mut Request) -> IronResult<Response>
    where C: MedalConnection + std::marker::Send + 'static {
    use params::{Params, Value};
    println!("{:?}", req.url.query().unwrap_or(""));

    let oauth_id = req.expect_str("oauthid")?;
    let school_id = req.get_str("schoolid");

    let (client_id, client_secret, access_token_url, user_data_url, school_data_url, school_data_secret) = {
        let config = req.get::<Read<SharedConfiguration>>().unwrap();

        let mut result: Option<(String, String, String, String, Option<String>, Option<String>)> = None;

        if let Some(ref oauth_providers) = config.oauth_providers {
            for oauth_provider in oauth_providers {
                if oauth_provider.provider_id == oauth_id {
                    result = Some((oauth_provider.client_id.clone(),
                                   oauth_provider.client_secret.clone(),
                                   oauth_provider.access_token_url.clone(),
                                   oauth_provider.user_data_url.clone(),
                                   oauth_provider.school_data_url.clone(),
                                   oauth_provider.school_data_secret.clone()));
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

    let (_state, _scope, code): (String, String, String) = {
        let map = req.get_ref::<Params>().unwrap();

        match (map.find(&["state"]), map.find(&["scope"]), map.find(&["code"])) {
            (Some(&Value::String(ref state)), Some(&Value::String(ref scope)), Some(&Value::String(ref code)))
                if state == "42" =>
            {
                (state.clone(), scope.clone(), code.clone())
            }
            _ => return Ok(Response::with(iron::status::NotFound)),
        }
    };

    let client = reqwest::Client::new();
    let params = [("code", code), ("grant_type", "authorization_code".to_string())];
    // TODO: This can fail if to much time has passed
    let res = client.post(&access_token_url).basic_auth(client_id, Some(client_secret)).form(&params).send();
    let access: OAuthAccess = res.expect("network error").json().expect("malformed json");

    let res = client.get(&user_data_url).bearer_auth(access.access_token).send();
    let mut user_data: OAuthUserData = res.expect("network error").json().expect("malformed json");

    //let mut school_data: Option<Vec<OAuthSchoolData>> = None;
    if let Some(school_data_url) = school_data_url {
        if let SchoolIdOrSchoolIds::SchoolIds(school_ids) = user_data.schoolId.unwrap() {
            let school_infos: Vec<(String, String)> =
                school_ids.into_iter()
                          .map(|school_id| {
                              use sha2::{Digest, Sha512};
                              let mut hasher = Sha512::default();
                              let string_to_hash = format!("{}{}", school_id, school_data_secret.as_ref().unwrap());
                              println!("{}", string_to_hash);
                              hasher.input(string_to_hash.as_bytes());
                              let hashed_string = hasher.result();
                              let hash: String = format!("{:X?}", hashed_string).chars()
                                                                                .filter(|c| c.is_ascii_alphanumeric())
                                                                                .collect();
                              println!("{}", hash);

                              let params = [("schoolId", school_id.clone()), ("hash", hash)];
                              let res = client.post(&school_data_url).form(&params).send();
                              let school_data: OAuthSchoolData =
                                  res.expect("network error").json().expect("malformed json");

                              println!("{:?}", school_data);

                              (school_id,
                               school_data.name
                                          .or(school_data.error)
                                          .unwrap_or_else(|| "Information missing".to_string()))
                          })
                          .collect();

            if school_id.is_none() {
                let mut data = json_val::Map::new();
                data.insert("schools".to_string(), to_json(&school_infos));
                println!("{:?}", req.url.query().unwrap_or(""));
                data.insert("query".to_string(), to_json(&req.url.query().unwrap_or("")));
                let mut resp = Response::new();
                resp.set_mut(Template::new(&"oauth_school_selector", data)).set_mut(status::Ok);
                return Ok(resp);
            }
        }
    }

    if let Some(id) = user_data.userID {
        user_data.userId_int = Some(id);
    }
    if let Some(id) = user_data.userId {
        user_data.userId_int = Some(id);
    }

    use core::{UserSex, UserType};

    let user_data = core::ForeignUserData { foreign_id: user_data.userId_int.unwrap(), // todo: don't unwrap here
                                            foreign_type: match user_data.userType.as_ref() {
                                                "a" | "A" => UserType::Admin,
                                                "t" | "T" => UserType::Teacher,
                                                "s" | "S" => UserType::User,
                                                _ => UserType::User,
                                            },
                                            sex: match user_data.gender.as_ref() {
                                                "m" | "M" => UserSex::Male,
                                                "f" | "F" | "w" | "W" => UserSex::Female,
                                                "?" => UserSex::Unknown,
                                                _ => UserSex::Unknown,
                                            },
                                            firstname: user_data.firstName,
                                            lastname: user_data.lastName };
    let user_type = user_data.foreign_type;

    let oauthloginresult = {
        // hier ggf. Daten aus dem Request holen
        let mutex = req.get::<Write<SharedDatabaseConnection<C>>>().unwrap();
        let conn = mutex.lock().unwrap_or_else(|e| e.into_inner());

        // Antwort erstellen und zurücksenden
        core::login_oauth(&*conn, user_data, oauth_id)
        /*let mut data = json_val::Map::new();
        data.insert("reason".to_string(), to_json(&"Not implemented".to_string()));
        ("profile", data)*/
    };

    match oauthloginresult {
        // Login successful
        Ok((sessionkey, redirectprofile)) => {
            req.session().set(SessionToken { token: sessionkey }).unwrap();

            if user_type == UserType::User && redirectprofile {
                Ok(Response::with((status::Found,
                                   Redirect(iron::Url::parse(&format!("{}?status=firstlogin",
                                                                      &url_for!(req, "profile"))).unwrap()))))
            } else {
                Ok(Response::with((status::Found, Redirect(url_for!(req, "greet")))))
            }
            // TODO: Make Response depend on a) has user logged in before? How long ago
            //       -> Ask for data if longer than 1 Month ago
        }
        // Login failed
        Err((template, data)) => {
            let mut resp = Response::new();
            resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
            Ok(resp)
        }
    }
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
        contestresults: get "/contest/:contestid/result/" => contestresults::<C>,
        contestresults_download: get "/contest/:contestid/result/download" => contestresults_download::<C>,
        contest_post: post "/contest/:contestid" => contest_post::<C>,
        login: get "/login" => login::<C>,
        login_post: post "/login" => login_post::<C>,
        login_code_post: post "/clogin" => login_code_post::<C>,
        logout: get "/logout" => logout::<C>,
        signup: get "/signup" => signup::<C>,
        signup_post: post "/signup" => signup_post::<C>,
        subm: get "/submission/:taskid" => submission::<C>,
        subm_post: post "/submission/:taskid" => submission_post::<C>,
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
        oauth: get "/oauth/:oauthid/" => oauth::<C>,
        oauth_school: get "/oauth/:oauthid/:schoolid" => oauth::<C>,
        check_cookie: get "/cookie" => cookie_warning,
        dbstatus: get "/dbstatus" => dbstatus::<C>,
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
