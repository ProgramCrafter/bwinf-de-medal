//extern crate serde;

use std::path::Path;

use iron::prelude::*;
use iron_sessionstorage::traits::*;

use iron::modifiers::Redirect;
use iron::modifiers::RedirectRaw;
use iron::{status, AfterMiddleware};

use mount::Mount;
use router::Router;
use staticfile::Static;

use iron_sessionstorage::backends::SignedCookieBackend;
use iron_sessionstorage::SessionStorage;
use persistent::Write;
use rusqlite::Connection;
use urlencoded::{UrlEncodedBody, UrlEncodedQuery};

pub use handlebars_iron::handlebars::to_json;
use handlebars_iron::{DirectorySource, HandlebarsEngine, Template};

use iron;
use iron_sessionstorage;
use reqwest;
use rusqlite;

pub use serde_json::value as json_val;

use iron::typemap::Key;

static TASK_DIR: &'static str = "tasks";

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
    ( $x:expr , $r:expr , $($y:expr),* ) => {
        {
            let mutex = $r.get::<Write<SharedDatabaseConnection>>().unwrap();
            let conn = mutex.lock().unwrap_or_else(|e| e.into_inner());
            $x(&*conn, $($y),*)
        }
    };
}

struct ErrorReporter;
impl AfterMiddleware for ErrorReporter {
    fn catch(&self, _: &mut Request, err: IronError) -> IronResult<Response> {
        println!("{}", err);
        Err(err)
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

use iron::middleware::{AroundMiddleware, Handler};
use rand::{distributions::Alphanumeric, thread_rng, Rng};

pub struct CookieDistributor {}

impl CookieDistributor {
    pub fn new() -> Self { Self {} }
}

impl AroundMiddleware for CookieDistributor {
    fn around(self, handler: Box<Handler>) -> Box<Handler> {
        Box::new(move |req: &mut Request| -> IronResult<Response> {
            if req.session().get::<SessionToken>().expect("blub...").is_none() {
                let session_token: String = thread_rng().sample_iter(&Alphanumeric).take(10).collect();
                req.session().set(SessionToken { token: session_token }).unwrap();
            }
            handler.handle(req)
        })
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
                use rand::{thread_rng, Rng};

                let new_session_key: String = thread_rng().sample_iter(&Alphanumeric).take(28).collect();
                self.session().set(SessionToken { token: new_session_key }).unwrap();
                Err(IronError { error: Box::new(SessionError { message: "No valid session found, redirecting to cookie page".to_string() }),
                                response: Response::with((status::Found, RedirectRaw(format!("/cookie?{}", self.url.path().join("/"))))) })
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
}

use functions;

struct AugMedalError<'c, 'a: 'c, 'b: 'c + 'a>(functions::MedalError, &'c mut Request<'a, 'b>);

impl<'c, 'a, 'b> From<AugMedalError<'c, 'a, 'b>> for IronError {
    fn from(AugMedalError(me, req): AugMedalError<'c, 'a, 'b>) -> Self {
        match me {
            functions::MedalError::NotLoggedIn => {
                IronError { error: Box::new(SessionError { message:
                                                               "Not Logged in, redirecting to login page".to_string() }),
                            response: Response::with((status::Found,
                                                      RedirectRaw(format!("/login?{}", req.url.path().join("/"))))) }
            }
            functions::MedalError::AccessDenied => {
                IronError { error: Box::new(SessionError { message: "Access denied".to_string() }),
                            response: Response::with(status::Forbidden) }
            }
            functions::MedalError::CsrfCheckFailed => {
                IronError { error: Box::new(SessionError { message: "CSRF Error".to_string() }),
                            response: Response::with(status::Forbidden) }
            }
            functions::MedalError::SessionTimeout => {
                IronError { error: Box::new(SessionError { message: "Session timed out".to_string() }),
                            response: Response::with(status::Forbidden) }
            }
            functions::MedalError::DatabaseError => {
                IronError { error: Box::new(SessionError { message: "Database Error".to_string() }),
                            response: Response::with(status::Forbidden) }
            }
            functions::MedalError::PasswordHashingError => {
                IronError { error: Box::new(SessionError { message: "Error hashing the passwords".to_string() }),
                            response: Response::with(status::Forbidden) }
            }
            functions::MedalError::UnmatchedPasswords => {
                IronError { error: Box::new(SessionError { message:
                                                               "The two passwords did not match.".to_string() }),
                            response: Response::with(status::Forbidden) }
            }
        }
    }
}

trait RequestAugmentMedalError<'c, 'a: 'c, 'b: 'c + 'a, R> {
    fn aug(self, req: &'c mut Request<'a, 'b>) -> Result<R, AugMedalError<'c, 'a, 'b>>;
}
impl<'c, 'a: 'c, 'b: 'c + 'a, T> RequestAugmentMedalError<'c, 'a, 'b, T> for Result<T, functions::MedalError> {
    fn aug(self, req: &'c mut Request<'a, 'b>) -> Result<T, AugMedalError<'c, 'a, 'b>> {
        self.map_err(move |me| AugMedalError(me, req))
    }
}

fn greet_personal(req: &mut Request) -> IronResult<Response> {
    let session_token = req.get_session_token();
    // hier ggf. Daten aus dem Request holen

    let (self_url, oauth_url) = {
        let mutex = req.get::<Write<SharedConfiguration>>().unwrap();
        let config = mutex.lock().unwrap_or_else(|e| e.into_inner());
        (config.self_url.clone(), config.oauth_url.clone())
    };

    let (template, data) = {
        // hier ggf. Daten aus dem Request holen
        let mutex = req.get::<Write<SharedDatabaseConnection>>().unwrap();
        let conn = mutex.lock().unwrap_or_else(|e| e.into_inner());

        // Antwort erstellen und zurücksenden
        functions::index(&*conn, session_token, (self_url, oauth_url))
    };
    // Daten verarbeiten

    // Antwort erstellen und zurücksenden
    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}

fn contests(req: &mut Request) -> IronResult<Response> {
    let (template, data) = with_conn![functions::show_contests, req,];

    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}

fn contest(req: &mut Request) -> IronResult<Response> {
    let contest_id = req.expect_int::<u32>("contestid")?;
    let session_token = req.require_session_token()?;

    let (template, data) = with_conn![functions::show_contest, req, contest_id, session_token].aug(req)?;

    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}

fn contestresults(req: &mut Request) -> IronResult<Response> {
    let contest_id = req.expect_int::<u32>("contestid")?;
    let session_token = req.require_session_token()?;

    let (template, data) = with_conn![functions::show_contest_results, req, contest_id, session_token].aug(req)?;

    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}

fn contest_post(req: &mut Request) -> IronResult<Response> {
    let contest_id = req.expect_int::<u32>("contestid")?;
    let session_token = req.expect_session_token()?;

    let csrf_token = {
        let formdata = itry!(req.get_ref::<UrlEncodedBody>());
        iexpect!(formdata.get("csrftoken"))[0].to_owned()
    };

    // TODO: Was mit dem Result?
    with_conn![functions::start_contest, req, contest_id, session_token, csrf_token].aug(req)?;

    Ok(Response::with((status::Found, Redirect(url_for!(req, "contest", "contestid" => format!("{}",contest_id))))))
}

fn login(req: &mut Request) -> IronResult<Response> {
    let (self_url, oauth_url) = {
        let mutex = req.get::<Write<SharedConfiguration>>().unwrap();
        let config = mutex.lock().unwrap_or_else(|e| e.into_inner());

        (config.self_url.clone(), config.oauth_url.clone())
    };

    let mut data = json_val::Map::new();
    data.insert("self_url".to_string(), to_json(&self_url));
    data.insert("oauth_url".to_string(), to_json(&oauth_url));

    let mut resp = Response::new();
    resp.set_mut(Template::new("login", data)).set_mut(status::Ok);
    Ok(resp)
}

fn login_post(req: &mut Request) -> IronResult<Response> {
    let logindata = {
        let formdata = itry!(req.get_ref::<UrlEncodedBody>());
        (iexpect!(formdata.get("username"))[0].to_owned(), iexpect!(formdata.get("password"))[0].to_owned())
    };

    // TODO: Submit current session to login

    let loginresult = with_conn![functions::login, req, logindata];

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

fn login_code_post(req: &mut Request) -> IronResult<Response> {
    let code = {
        let formdata = itry!(req.get_ref::<UrlEncodedBody>());
        iexpect!(formdata.get("code"))[0].to_owned()
    };

    // TODO: Submit current session to login

    let loginresult = with_conn![functions::login_with_code, req, code];
    println!("aa");

    match loginresult {
        // Login successful
        Ok(Ok(sessionkey)) => {
            req.session().set(SessionToken { token: sessionkey }).unwrap();
            Ok(Response::with((status::Found, Redirect(url_for!(req, "greet")))))
        }
        Ok(Err(sessionkey)) => {
            req.session().set(SessionToken { token: sessionkey }).unwrap();
            Ok(Response::with((status::Found, Redirect(url_for!(req, "profile")))))
        }
        // Login failed
        Err((template, data)) => {
            println!("bb");
            let mut resp = Response::new();
            resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
            Ok(resp)
        }
    }
}

fn logout(req: &mut Request) -> IronResult<Response> {
    let session_token = req.get_session_token();

    println!("Loggin out session {:?}", session_token);

    with_conn![functions::logout, req, session_token];

    Ok(Response::with((status::Found, Redirect(url_for!(req, "greet")))))
}

fn submission(req: &mut Request) -> IronResult<Response> {
    let task_id = req.expect_int::<u32>("taskid")?;
    let session_token = req.expect_session_token()?;
    let subtask: Option<String> = (|| -> Option<String> {
        req.get_ref::<UrlEncodedQuery>().ok()?.get("subtask")?.get(0).map(|x| x.to_owned())
    })();

    println!("{}", task_id);

    let result = with_conn![functions::load_submission, req, task_id, session_token, subtask];

    match result {
        Ok(data) => Ok(Response::with((status::Ok, mime!(Application / Json), format!("{}", data)))),
        Err(_) => Ok(Response::with((status::BadRequest, mime!(Application / Json), format!("{{}}")))),
    }
}

fn submission_post(req: &mut Request) -> IronResult<Response> {
    let task_id = req.expect_int::<u32>("taskid")?;
    let session_token = req.expect_session_token()?;
    let (csrf_token, data, grade, subtask) = {
        let formdata = iexpect!(req.get_ref::<UrlEncodedBody>().ok());
        (iexpect!(formdata.get("csrf"),(status::BadRequest, mime!(Text/Html), format!("400 Bad Request")))[0].to_owned(),
         iexpect!(formdata.get("data"),(status::BadRequest, mime!(Text/Html), format!("400 Bad Request")))[0].to_owned(),
         iexpect!(formdata.get("grade").unwrap_or(&vec!["0".to_owned()])[0].parse::<u8>().ok(),(status::BadRequest, mime!(Text/Html), format!("400 Bad Request"))),
         formdata.get("subtask").map(|x| x[0].to_owned()),
        )
    };
    println!("{}", data);
    println!("{}", task_id);
    println!("{}", grade);

    let result = with_conn![functions::save_submission, req, task_id, session_token, csrf_token, data, grade, subtask];

    match result {
        Ok(_) => Ok(Response::with((status::Ok, mime!(Application / Json), format!("{{}}")))),
        Err(_) => Ok(Response::with((status::BadRequest, mime!(Application / Json), format!("{{}}")))),
    }
}

fn task(req: &mut Request) -> IronResult<Response> {
    let task_id = req.expect_int::<u32>("taskid")?;
    let session_token = req.require_session_token()?;

    println!("{}", task_id);

    let (template, data) = with_conn![functions::show_task, req, task_id, session_token].aug(req)?;

    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}

fn groups(req: &mut Request) -> IronResult<Response> {
    let session_token = req.require_session_token()?;

    let (template, data) = with_conn![functions::show_groups, req, session_token].aug(req)?;

    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}

fn group(req: &mut Request) -> IronResult<Response> {
    let group_id = req.expect_int::<u32>("groupid")?;
    let session_token = req.require_session_token()?;

    let (template, data) = with_conn![functions::show_group, req, group_id, session_token].aug(req)?;

    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}

fn group_post(req: &mut Request) -> IronResult<Response> {
    let group_id = req.expect_int::<u32>("groupid")?;
    let session_token = req.expect_session_token()?;

    //TODO: use result?
    with_conn![functions::modify_group, req, group_id, session_token].aug(req)?;

    Ok(Response::with((status::Found, Redirect(url_for!(req, "group", "groupid" => format!("{}",group_id))))))
}

fn new_group(req: &mut Request) -> IronResult<Response> {
    let session_token = req.require_session_token()?;

    let (csrf, name, tag) = {
        let formdata = iexpect!(req.get_ref::<UrlEncodedBody>().ok());
        (iexpect!(formdata.get("csrf"),(status::BadRequest, mime!(Text/Html), format!("400 Bad Request")))[0].to_owned(),
         iexpect!(formdata.get("name"),(status::BadRequest, mime!(Text/Html), format!("400 Bad Request")))[0].to_owned(),
         iexpect!(formdata.get("tag"),(status::BadRequest, mime!(Text/Html), format!("400 Bad Request")))[0].to_owned())
    };
    println!("{}", csrf);
    println!("{}", name);

    let group_id = with_conn![functions::add_group, req, session_token, csrf, name, tag].aug(req)?;

    Ok(Response::with((status::Found, Redirect(url_for!(req, "group", "groupid" => format!("{}",group_id))))))
}

fn profile(req: &mut Request) -> IronResult<Response> {
    let session_token = req.require_session_token()?;
    let query_string = req.url.query().map(|s| s.to_string());

    let (template, data) = with_conn![functions::show_profile, req, session_token, None, query_string].aug(req)?;

    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}

fn profile_post(req: &mut Request) -> IronResult<Response> {
    let session_token = req.expect_session_token()?;
    let (csrf_token, firstname, lastname, street, zip, city, pwd, pwd_repeat, grade) = {
        let formdata = itry!(req.get_ref::<UrlEncodedBody>());
        (iexpect!(formdata.get("csrftoken"))[0].to_owned(),
         iexpect!(formdata.get("firstname"))[0].to_owned(),
         iexpect!(formdata.get("lastname"))[0].to_owned(),
         formdata.get("street").map(|x| x[0].to_owned()),
         formdata.get("zip").map(|x| x[0].to_owned()),
         formdata.get("city").map(|x| x[0].to_owned()),
         formdata.get("password").map(|x| x[0].to_owned()),
         formdata.get("password_repeat").map(|x| x[0].to_owned()),
         iexpect!(formdata.get("grade"))[0].parse::<u8>().unwrap_or(0))
    };

    let profilechangeresult = with_conn![functions::edit_profile,
                                         req,
                                         session_token,
                                         None,
                                         csrf_token,
                                         firstname,
                                         lastname,
                                         street,
                                         zip,
                                         city,
                                         pwd,
                                         pwd_repeat,
                                         grade].aug(req)?;

    Ok(Response::with((status::Found,
                       Redirect(iron::Url::parse(&format!("{}?status={:?}",
                                                          &url_for!(req, "profile"),
                                                          profilechangeresult)).unwrap()))))
}

fn user(req: &mut Request) -> IronResult<Response> {
    let user_id = req.expect_int::<u32>("userid")?;
    let session_token = req.expect_session_token()?;
    let query_string = req.url.query().map(|s| s.to_string());

    let (template, data) =
        with_conn![functions::show_profile, req, session_token, Some(user_id), query_string].aug(req)?;

    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}

fn user_post(req: &mut Request) -> IronResult<Response> {
    let user_id = req.expect_int::<u32>("userid")?;
    let session_token = req.expect_session_token()?;
    let (csrf_token, firstname, lastname, street, zip, city, pwd, pwd_repeat, grade) = {
        let formdata = itry!(req.get_ref::<UrlEncodedBody>());
        (iexpect!(formdata.get("csrftoken"))[0].to_owned(),
         iexpect!(formdata.get("firstname"))[0].to_owned(),
         iexpect!(formdata.get("lastname"))[0].to_owned(),
         formdata.get("street").map(|x| x[0].to_owned()),
         formdata.get("zip").map(|x| x[0].to_owned()),
         formdata.get("city").map(|x| x[0].to_owned()),
         formdata.get("password").map(|x| x[0].to_owned()),
         formdata.get("password_repeat").map(|x| x[0].to_owned()),
         iexpect!(formdata.get("grade"))[0].parse::<u8>().unwrap_or(0))
    };

    let profilechangeresult = with_conn![functions::edit_profile,
                                         req,
                                         session_token,
                                         Some(user_id),
                                         csrf_token,
                                         firstname,
                                         lastname,
                                         street,
                                         zip,
                                         city,
                                         pwd,
                                         pwd_repeat,
                                         grade].aug(req)?;

    Ok(Response::with((status::Found,
                       Redirect(iron::Url::parse(&format!("{}?status={:?}",
                                                          &url_for!(req, "user", "userid" => format!("{}",user_id)),
                                                          profilechangeresult)).unwrap()))))
    //old:   Ok(Response::with((status::Found, Redirect(url_for!(req, "user", "userid" => format!("{}",user_id))))))
}

#[derive(Deserialize, Debug)]
struct OAuthAccess {
    access_token: String,
    token_type: String,
    refresh_token: String,
    expires: Option<u32>,    // documented as 'expires_in'
    expires_in: Option<u32>, // sent as 'expires'
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
    userId_int: Option<u32>,
}

fn oauth(req: &mut Request) -> IronResult<Response> {
    use params::{Params, Value};
    use reqwest::header;

    let (client_id, client_secret, access_token_url, user_data_url) = {
        let mutex = req.get::<Write<SharedConfiguration>>().unwrap();
        let config = mutex.lock().unwrap_or_else(|e| e.into_inner());
        if let (Some(id), Some(secret), Some(atu), Some(udu)) = (&config.oauth_client_id,
                                                                 &config.oauth_client_secret,
                                                                 &config.oauth_access_token_url,
                                                                 &config.oauth_user_data_url)
        {
            (id.clone(), secret.clone(), atu.clone(), udu.clone())
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

    let client = reqwest::Client::new().unwrap();
    let params = [("code", code), ("grant_type", "authorization_code".to_string())];
    let res = client.post(&access_token_url)
                    .header(header::Authorization(header::Basic { username: client_id, password: Some(client_secret) }))
                    .form(&params)
                    .send();
    let access: OAuthAccess = res.expect("network error").json().expect("malformed json");

    let res = client.post(&user_data_url)
                    .header(header::Authorization(header::Bearer { token: access.access_token }))
                    .form(&params)
                    .send();
    let mut user_data: OAuthUserData = res.expect("network error").json().expect("malformed json");

    if let Some(ref id) = user_data.userID {
        user_data.userId_int = Some(id.parse::<u32>().unwrap());
    }
    if let Some(ref id) = user_data.userId {
        user_data.userId_int = Some(id.parse::<u32>().unwrap());
    }

    use functions::{UserGender, UserType};

    let user_data = functions::ForeignUserData { foreign_id: user_data.userId_int.unwrap(),
                                                 foreign_type: match user_data.userType.as_ref() {
                                                     "a" | "A" => UserType::Admin,
                                                     "t" | "T" => UserType::Teacher,
                                                     "s" | "S" | _ => UserType::User,
                                                 },
                                                 gender: match user_data.gender.as_ref() {
                                                     "m" | "M" => UserGender::Male,
                                                     "f" | "F" | "w" | "W" => UserGender::Female,
                                                     "?" | _ => UserGender::Unknown,
                                                 },
                                                 firstname: user_data.firstName,
                                                 lastname: user_data.lastName };

    let oauthloginresult = {
        // hier ggf. Daten aus dem Request holen
        let mutex = req.get::<Write<SharedDatabaseConnection>>().unwrap();
        let conn = mutex.lock().unwrap_or_else(|e| e.into_inner());

        // Antwort erstellen und zurücksenden
        functions::login_oauth(&*conn, user_data)
        /*let mut data = json_val::Map::new();
        data.insert("reason".to_string(), to_json(&"Not implemented".to_string()));
        ("profile", data)*/
    };

    match oauthloginresult {
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

// Share Database connection between workers
#[derive(Copy, Clone)]
pub struct SharedDatabaseConnection;
impl Key for SharedDatabaseConnection {
    type Value = rusqlite::Connection;
}

// Share Configuration between workers
#[derive(Copy, Clone)]
pub struct SharedConfiguration;
impl Key for SharedConfiguration {
    type Value = ::Config;
}

#[cfg(feature = "watch")]
pub fn get_handlebars_engine() -> impl AfterMiddleware {
    // HandlebarsEngine will look up all files with "./examples/templates/**/*.hbs"
    let mut hbse = HandlebarsEngine::new();
    hbse.add(Box::new(DirectorySource::new("./templates/", ".hbs")));

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
pub fn get_handlebars_engine() -> impl AfterMiddleware {
    // HandlebarsEngine will look up all files with "./examples/templates/**/*.hbs"
    let mut hbse = HandlebarsEngine::new();
    hbse.add(Box::new(DirectorySource::new("./templates/", ".hbs")));

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

pub fn start_server(conn: Connection, config: ::Config) -> iron::error::HttpResult<iron::Listening> {
    let router = router!(
        greet: get "/" => greet_personal,
        contests: get "/contest/" => contests,
        contest: get "/contest/:contestid" => contest,
        contestresults: get "/contest/:contestid/result/" => contestresults,
        contest_post: post "/contest/:contestid" => contest_post,
        login: get "/login" => login,
        login_post: post "/login" => login_post,
        login_code_post: post "/clogin" => login_code_post,
        logout: get "/logout" => logout,
        subm: get "/submission/:taskid" => submission,
        subm_post: post "/submission/:taskid" => submission_post,
        subm_load: get "/load/:taskid" => submission,
        subm_save: post "/save/:taskid" => submission_post,
        groups: get "/group/" => groups,
        groups: post "/group/" => new_group,
        group: get "/group/:groupid" => group,
        group_post: post "/group" => group_post,
        profile: get "/profile" => profile,
        profile_post: post "/profile" => profile_post,
        user: get "/user/:userid" => user,
        user_post: post "/user/:userid" => user_post,
        task: get "/task/:taskid" => task,
        oauth: get "/oauth" => oauth,
        check_cookie: get "/cookie" => cookie_warning,
    );

    let my_secret = b"verysecret".to_vec();

    let mut mount = Mount::new();

    // Serve the shared JS/CSS at /
    mount.mount("/static/", Static::new(Path::new("static")));
    mount.mount("/tasks/", Static::new(Path::new(TASK_DIR)));
    mount.mount("/", router);

    let mut ch = Chain::new(mount);

    ch.link(Write::<SharedDatabaseConnection>::both(conn));
    ch.link(Write::<SharedConfiguration>::both(config.clone()));
    ch.link_around(CookieDistributor::new());
    ch.link_around(SessionStorage::new(SignedCookieBackend::new(my_secret)));

    ch.link_after(get_handlebars_engine());
    ch.link_after(ErrorReporter);

    let socket_addr = format!("{}:{}", config.host.unwrap(), config.port.unwrap());

    let srvr = Iron::new(ch).http(&socket_addr);
    println!("Listening on {}.", &socket_addr);
    srvr
}
