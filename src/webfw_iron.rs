
//extern crate serde;



use std::path::Path;

use iron_sessionstorage::traits::*;

use iron::prelude::*;
use iron::{status, AfterMiddleware};
use iron::modifiers::Redirect;
use iron::modifiers::RedirectRaw;

use mount::Mount;
use router::Router;
use staticfile::Static;

use iron_sessionstorage::SessionStorage;
use iron_sessionstorage::backends::SignedCookieBackend;
use rusqlite::Connection;
use urlencoded::{UrlEncodedBody,UrlEncodedQuery};
use persistent::Write;

use handlebars_iron::{HandlebarsEngine,DirectorySource,Template};
pub use handlebars_iron::handlebars::to_json;

use iron::prelude::*;
use iron_sessionstorage::traits::*;

use iron_sessionstorage;
use iron;
use reqwest;
use rusqlite;

pub use serde_json::value as json_val;

use iron::typemap::Key;


static DB_FILE: &'static str = "medal.db";
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



struct ErrorReporter;
impl AfterMiddleware for ErrorReporter {
    fn catch(&self, _: &mut Request, err: IronError) -> IronResult<Response> {
        println!("{}", err);
        Err(err)
    }
}

#[derive(Debug)]
struct SessionToken {
    token: String
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


#[derive(Debug)]
struct SessionError {
    message: String
}
impl ::std::error::Error for SessionError {
    fn description(&self) -> &str {
        &self.message
    }
}

impl ::std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

fn get_session_token(req: &mut Request) -> Option<String> {
    let session_token = SessionRequestExt::session(req).get::<SessionToken>().unwrap();
    (|st: Option<SessionToken>| -> Option<String> {Some(st?.token)}) (session_token)
}

fn require_session_token(req: &mut Request) -> IronResult<String> {
    match  SessionRequestExt::session(req).get::<SessionToken>().unwrap() {
        Some(SessionToken { token: session }) => Ok(session),
        _ => {
            let sessionkey = String::from("abced");
            req.session().set(SessionToken { token: sessionkey }).unwrap();
            println!("{}", req.url);
            Err(IronError { error: Box::new(SessionError { message: "No valid session found".to_string() }),
                            response: Response::with((status::Found, RedirectRaw(format!("/cookie?{}", req.url.path().join("/"))))) })
        }
    }
}


use ::functions;


fn greet(req: &mut Request) -> IronResult<Response> {
    // hier ggf. Daten aus dem Request holen

    // Daten verarbeiten
    let (template, data) = functions::blaa();

    // Antwort erstellen und zurücksenden  
    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}


/*
    self.session().get::<SessionToken>().unwrap();
    self.session().set(SessionToken { token: token.clone() }).unwrap();
*/
fn greet_personal(req: &mut Request) -> IronResult<Response> {
    let session_token = get_session_token(req);
    // hier ggf. Daten aus dem Request holen

    let (template, data) = {
        // hier ggf. Daten aus dem Request holen
        let mutex = req.get::<Write<SharedDatabaseConnection>>().unwrap();
        let conn = mutex.lock().unwrap_or_else(|e| e.into_inner());

        // Antwort erstellen und zurücksenden   
        functions::index(&*conn, session_token)
    };
    // Daten verarbeiten
//    let (template, data) = functions::blaa();

    // Antwort erstellen und zurücksenden  
    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}

fn contests(req: &mut Request) -> IronResult<Response> {
    let (template, data) = {
        // hier ggf. Daten aus dem Request holen
        let mutex = req.get::<Write<SharedDatabaseConnection>>().unwrap();
        let conn = mutex.lock().unwrap_or_else(|e| e.into_inner());

        // Antwort erstellen und zurücksenden   
        functions::show_contests(&*conn)
    };
    
    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}

fn contest(req: &mut Request) -> IronResult<Response> {
    let contest_id = req.extensions.get::<Router>().unwrap().find("contestid").unwrap_or("").parse::<u32>().unwrap_or(0);
    //let session_token = SessionRequestExt::session(req).get::<SessionToken>().unwrap().unwrap(); // TODO: Was ist ohne session?
    let session_token = require_session_token(req)?;

    let (template, data) = {
        // hier ggf. Daten aus dem Request holen
        let mutex = req.get::<Write<SharedDatabaseConnection>>().unwrap();
        let conn = mutex.lock().unwrap_or_else(|e| e.into_inner());

        // Antwort erstellen und zurücksenden   
        functions::show_contest(&*conn, contest_id,  session_token)
    };

    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}

fn contest_post(req: &mut Request) -> IronResult<Response> {
    let contest_id = req.extensions.get::<Router>().unwrap().find("contestid").unwrap_or("").parse::<u32>().unwrap_or(0);
    let session_token = SessionRequestExt::session(req).get::<SessionToken>().unwrap().unwrap();
    let csrf_token = {
        let formdata = itry!(req.get_ref::<UrlEncodedBody>());
        iexpect!(formdata.get("csrftoken"))[0].to_owned()
    };

    let startcontestresult = {
        let mutex = req.get::<Write<SharedDatabaseConnection>>().unwrap();
        
        let conn = mutex.lock().unwrap_or_else(|e| e.into_inner());

        // Antwort erstellen und zurücksenden   
        functions::start_contest(&*conn, contest_id, session_token.token, csrf_token)
    };

    match startcontestresult {
        // Start successful
        Ok(()) => {
            Ok(Response::with((status::Found, Redirect(url_for!(req, "contest", "contestid" => format!("{}",contest_id))))))
        },
        // Start failed
        Err((template, data)) => {
            let mut resp = Response::new();
            resp.set_mut(Template::new(&template, data)).set_mut(status::Forbidden);
            Ok(resp)
        }
    }
}

fn login(_: &mut Request) -> IronResult<Response> {
    let mut resp = Response::new();
    resp.set_mut(Template::new("login", "".to_owned())).set_mut(status::Ok);
    Ok(resp)
}

fn login_post(req: &mut Request) -> IronResult<Response> {
    let logindata = {
        let formdata = itry!(req.get_ref::<UrlEncodedBody>());
        (iexpect!(formdata.get("username"))[0].to_owned(),
         iexpect!(formdata.get("password"))[0].to_owned())
    };

    let loginresult = {
        let mutex = req.get::<Write<SharedDatabaseConnection>>().unwrap();
        let conn = mutex.lock().unwrap_or_else(|e| e.into_inner());

        // Antwort erstellen und zurücksenden   
        functions::login(&*conn, logindata)
    };

    match loginresult {
        // Login successful
        Ok(sessionkey) => {
            req.session().set(SessionToken { token: sessionkey }).unwrap();
            Ok(Response::with((status::Found, Redirect(url_for!(req, "greet")))))
        },
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

    let loginresult = {
        let mutex = req.get::<Write<SharedDatabaseConnection>>().unwrap();
        let conn = mutex.lock().unwrap_or_else(|e| e.into_inner());

        // Antwort erstellen und zurücksenden   
        functions::login_with_code(&*conn, code)
    };
    println!("aa");

    match loginresult {
        // Login successful
        Ok(Ok(sessionkey)) => {
            req.session().set(SessionToken { token: sessionkey }).unwrap();
            Ok(Response::with((status::Found, Redirect(url_for!(req, "greet")))))
        },
        Ok(Err(sessionkey)) => {
            req.session().set(SessionToken { token: sessionkey }).unwrap();
            Ok(Response::with((status::Found, Redirect(url_for!(req, "profile")))))
        },
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
    let session_token = SessionRequestExt::session(req).get::<SessionToken>().unwrap();

    {
        let mutex = req.get::<Write<SharedDatabaseConnection>>().unwrap();
        let conn = mutex.lock().unwrap_or_else(|e| e.into_inner());

        println!("Loggin out session {:?}", session_token);
        functions::logout(&*conn, (|st: Option<SessionToken>| -> Option<String> {Some(st?.token)}) (session_token));
    };

    Ok(Response::with((status::Found, Redirect(url_for!(req, "greet")))))
}


fn submission(req: &mut Request) -> IronResult<Response> {
    let task_id = req.extensions.get::<Router>().unwrap().find("taskid").unwrap_or("").parse::<u32>().unwrap_or(0);
    let session_token = SessionRequestExt::session(req).get::<SessionToken>().unwrap().unwrap();
    
    println!("{}",task_id);

    let result = {
        let mutex = req.get::<Write<SharedDatabaseConnection>>().unwrap();
        let conn = mutex.lock().unwrap_or_else(|e| e.into_inner());
        functions::load_submission(&*conn, task_id, session_token.token)
    };

    match result {
        Ok(data) => Ok(Response::with((
            status::Ok,
            mime!(Application/Json),
            format!("{}", data)))),
        Err(_) => Ok(Response::with((
            status::BadRequest,
            mime!(Application/Json),
            format!("{{}}"))))
    }
}

fn submission_post(req: &mut Request) -> IronResult<Response> {
    let task_id = req.extensions.get::<Router>().unwrap().find("taskid").unwrap_or("").parse::<u32>().unwrap_or(0);
    let session_token = SessionRequestExt::session(req).get::<SessionToken>().unwrap().unwrap();
    let (csrf_token, data) = {
        let formdata = iexpect!(req.get_ref::<UrlEncodedBody>().ok());
        (iexpect!(formdata.get("csrf"),(status::BadRequest, mime!(Text/Html), format!("400 Bad Request")))[0].to_owned(),
         iexpect!(formdata.get("data"),(status::BadRequest, mime!(Text/Html), format!("400 Bad Request")))[0].to_owned())
        };
    println!("{}",data);
    println!("{}",task_id);
    
    let result = {
        let mutex = req.get::<Write<SharedDatabaseConnection>>().unwrap();
        let conn = mutex.lock().unwrap_or_else(|e| e.into_inner());
        functions::save_submission(&*conn, task_id, session_token.token, csrf_token, data)
    };
    
    match result {
        Ok(_) => Ok(Response::with((
            status::Ok,
            mime!(Application/Json),
            format!("{{}}")))),
        Err(_) => Ok(Response::with((
            status::BadRequest,
            mime!(Application/Json),
            format!("{{}}"))))
    }    
}

fn task(req: &mut Request) -> IronResult<Response> {
    let task_id = req.extensions.get::<Router>().unwrap().find("taskid").unwrap_or("").parse::<u32>().unwrap_or(0);
    // TODO: Make work without session
    let session_token = SessionRequestExt::session(req).get::<SessionToken>().unwrap().unwrap();
    
    println!("{}",task_id);

    let (template, data) = {
        let mutex = req.get::<Write<SharedDatabaseConnection>>().unwrap();
        let conn = mutex.lock().unwrap_or_else(|e| e.into_inner());
        functions::show_task(&*conn, task_id, session_token.token)
    };

    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}

fn groups(req: &mut Request) -> IronResult<Response> {
    let session_token = SessionRequestExt::session(req).get::<SessionToken>().unwrap().unwrap();
    
    let (template, data) = {
        // hier ggf. Daten aus dem Request holen
        let mutex = req.get::<Write<SharedDatabaseConnection>>().unwrap();
        let conn = mutex.lock().unwrap_or_else(|e| e.into_inner());

        // Antwort erstellen und zurücksenden   
        functions::show_groups(&*conn, session_token.token)
        /*let mut data = json_val::Map::new();
        data.insert("reason".to_string(), to_json(&"Not implemented".to_string()));
        ("groups", data)*/
    };
    
    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}

fn group(req: &mut Request) -> IronResult<Response> {
    let session_token = SessionRequestExt::session(req).get::<SessionToken>().unwrap().unwrap();
    let group_id = req.extensions.get::<Router>().unwrap().find("groupid").unwrap_or("").parse::<u32>().unwrap_or(0);
    
    let groupresult = {
        // hier ggf. Daten aus dem Request holen
        let mutex = req.get::<Write<SharedDatabaseConnection>>().unwrap();
        let conn = mutex.lock().unwrap_or_else(|e| e.into_inner());

        // Antwort erstellen und zurücksenden   
        functions::show_group(&*conn, group_id, session_token.token)
        /*let mut data = json_val::Map::new();
        data.insert("reason".to_string(), to_json(&"Not implemented".to_string()));
        ("group", data)*/
    };

     match groupresult {
        // Change successful
        Ok((template, data)) => {
            let mut resp = Response::new();
            resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
            Ok(resp)
        },
        // Change failed
        Err((template, data)) => {
            let mut resp = Response::new();
            resp.set_mut(Template::new(&template, data)).set_mut(status::Forbidden);
            Ok(resp)
        }
    }
}

fn group_post(req: &mut Request) -> IronResult<Response> {
    let session_token = SessionRequestExt::session(req).get::<SessionToken>().unwrap().unwrap();
    let group_id = req.extensions.get::<Router>().unwrap().find("groupid").unwrap_or("").parse::<u32>().unwrap_or(0);

    let changegroupresult = {
        // hier ggf. Daten aus dem Request holen
        let mutex = req.get::<Write<SharedDatabaseConnection>>().unwrap();
        let conn = mutex.lock().unwrap_or_else(|e| e.into_inner());

        // Antwort erstellen und zurücksenden   
        functions::modify_group(&*conn, group_id, session_token.token)
    };

    match changegroupresult {
        // Change successful
        Ok(()) => {
            Ok(Response::with((status::Found, Redirect(url_for!(req, "group", "groupid" => format!("{}",group_id))))))
        },
        // Change failed
        Err((template, data)) => {
            let mut resp = Response::new();
            resp.set_mut(Template::new(&template, data)).set_mut(status::Forbidden);
            Ok(resp)
        }
    }
}

fn new_group(req: &mut Request) -> IronResult<Response> {
    let session_token = SessionRequestExt::session(req).get::<SessionToken>().unwrap().unwrap();

    let (csrf, name, tag) = {
        let formdata = iexpect!(req.get_ref::<UrlEncodedBody>().ok());
        (iexpect!(formdata.get("csrf"),(status::BadRequest, mime!(Text/Html), format!("400 Bad Request")))[0].to_owned(),
         iexpect!(formdata.get("name"),(status::BadRequest, mime!(Text/Html), format!("400 Bad Request")))[0].to_owned(),
         iexpect!(formdata.get("tag"),(status::BadRequest, mime!(Text/Html), format!("400 Bad Request")))[0].to_owned())
    };
    println!("{}",csrf);
    println!("{}",name);

    let createresult = {
        let mutex = req.get::<Write<SharedDatabaseConnection>>().unwrap();
        let conn = mutex.lock().unwrap_or_else(|e| e.into_inner());
        functions::add_group(&*conn, session_token.token, csrf, name, tag)
    };

    
    match createresult {
        Ok(group_id) => {
            Ok(Response::with((status::Found, Redirect(url_for!(req, "group", "groupid" => format!("{}",group_id))))))
        },
        Err((template, data)) => {
            let mut resp = Response::new();
            resp.set_mut(Template::new(&template, data)).set_mut(status::BadRequest);
            Ok(resp)
        }
    }
}

fn profile(req: &mut Request) -> IronResult<Response> {
    let session_token = SessionRequestExt::session(req).get::<SessionToken>().unwrap().unwrap();
    
    let (template, data) = {
        // hier ggf. Daten aus dem Request holen
        let mutex = req.get::<Write<SharedDatabaseConnection>>().unwrap();
        let conn = mutex.lock().unwrap_or_else(|e| e.into_inner());

        // Antwort erstellen und zurücksenden   
        functions::show_profile(&*conn, session_token.token)
        /*let mut data = json_val::Map::new();
        data.insert("reason".to_string(), to_json(&"Not implemented".to_string()));
        ("profile", data)*/
    };
    
    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}

fn profile_post(req: &mut Request) -> IronResult<Response> {
    let session_token = SessionRequestExt::session(req).get::<SessionToken>().unwrap().unwrap();
    let (csrf_token, firstname, lastname, grade) = {
        let formdata = itry!(req.get_ref::<UrlEncodedBody>());
        (iexpect!(formdata.get("csrftoken"))[0].to_owned(),
         iexpect!(formdata.get("firstname"))[0].to_owned(),
         iexpect!(formdata.get("lastname"))[0].to_owned(),
         iexpect!(formdata.get("grade"))[0].parse::<u8>().unwrap_or(0))
         
    };
    
    let profilechangeresult  = {
        // hier ggf. Daten aus dem Request holen
        let mutex = req.get::<Write<SharedDatabaseConnection>>().unwrap();
        let conn = mutex.lock().unwrap_or_else(|e| e.into_inner());

        // Antwort erstellen und zurücksenden   
        functions::edit_profile(&*conn, session_token.token, csrf_token, firstname, lastname, grade)
        /*let mut data = json_val::Map::new();
        data.insert("reason".to_string(), to_json(&"Not implemented".to_string()));
        ("profile", data)*/
    };
    println!("hiiiiii");
    match profilechangeresult {
        Ok(()) => Ok(Response::with((status::Found, Redirect(url_for!(req, "profile"))))),
        Err((template, data)) => {
            let mut resp = Response::new();
            resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
            Ok(resp)
        }
    }
}

fn user(req: &mut Request) -> IronResult<Response> {
    let (template, data) = {
        // hier ggf. Daten aus dem Request holen
        let mutex = req.get::<Write<SharedDatabaseConnection>>().unwrap();
        let conn = mutex.lock().unwrap_or_else(|e| e.into_inner());

        // Antwort erstellen und zurücksenden   
        //functions::show_contests(&*conn)
        let mut data = json_val::Map::new();
        data.insert("reason".to_string(), to_json(&"Not implemented".to_string()));
        ("profile", data)
    };
    
    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}

#[derive(Deserialize, Debug)]
struct OAuthAccess {
    access_token:  String,
    token_type:    String,
    refresh_token: String,
    expires:       Option<u32>, // documented as 'expires_in'
    expires_in:    Option<u32>, // sent as 'expires'
}

#[derive(Deserialize, Debug)]
pub struct OAuthUserData {
    userID:      Option<String>, // documented as 'userId'
    userId:      Option<String>, // sent as 'userID'
    userType:    String,
    gender:      String,
    firstName:   String,
    lastName:    String,
    dateOfBirth: Option<String>, 
    eMail:       Option<String>,
    userId_int:  Option<u32>,
}

fn oauth(req: &mut Request) -> IronResult<Response> {
    use reqwest::header;
    use params::{Params, Value};
    use std::io::Read;

    let (client_id, client_secret, access_token_url, user_data_url) = {
        let mutex = req.get::<Write<SharedConfiguration>>().unwrap();
        let config = mutex.lock().unwrap_or_else(|e| e.into_inner());
        if let (Some(id), Some(secret), Some(atu), Some(udu))
            = (&config.oauth_client_id, &config.oauth_client_secret, &config.oauth_access_token_url, &config.oauth_user_data_url) {
            (id.clone(), secret.clone(), atu.clone(), udu.clone())
        } else {
            return Ok(Response::with(iron::status::NotFound))
        }
    };
    
    let (state, scope, code): (String, String, String) = {
        let map = req.get_ref::<Params>().unwrap();

        match (map.find(&["state"]),map.find(&["scope"]),map.find(&["code"])) {
            (Some(&Value::String(ref state)),Some(&Value::String(ref scope)),Some(&Value::String(ref code))) if state == "42" => {
                (state.clone(), scope.clone(), code.clone())
            },
            _ => return Ok(Response::with(iron::status::NotFound)),
        }
    };
                    
                
            let client = reqwest::Client::new().unwrap();
            let params = [("code", code), ("grant_type", "authorization_code".to_string())];
            let res = client.post(&access_token_url)
                .header(header::Authorization(header::Basic {
                    username: client_id,
                    password: Some(client_secret)}))
                .form(&params)
                .send();
            let access: OAuthAccess = res.expect("network error").json().expect("malformed json");

            let res = client.post(&user_data_url)
                .header(header::Authorization(header::Bearer{token: access.access_token}))
                .form(&params)
                .send();
            let mut user_data: OAuthUserData = res.expect("network error").json().expect("malformed json");

            if let Some(ref id) = user_data.userID {
                user_data.userId_int = Some(id.parse::<u32>().unwrap());
            }
            if let Some(ref id) = user_data.userId {
                user_data.userId_int = Some(id.parse::<u32>().unwrap());
            }

            use functions::{UserType, UserGender};

            let user_data = functions::ForeignUserData {
                foreign_id:   user_data.userId_int.unwrap(),
                foreign_type: match user_data.userType.as_ref() {
                    "a" | "A"     => UserType::Admin,
                    "t" | "T"     => UserType::Teacher,
                    "s" | "S" | _ => UserType::User,
                },
                gender: match user_data.gender.as_ref() {
                    "m" | "M"             => UserGender::Male,
                    "f" | "F" | "w" | "W" => UserGender::Female,
                    "?" | _               => UserGender::Unknown,
                },
                firstname:   user_data.firstName,
                lastname:    user_data.lastName,
            };

            
            let oauthloginresult  = {
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
                },
                // Login failed
                Err((template, data)) => {
                    let mut resp = Response::new();
                    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
                    Ok(resp)
                }
            }
            
            // Ok(Response::with((iron::status::Ok, format!("{:?}", user_data))))

    
    /*println!("oauth");
    
    let mut data = json_val::Map::new();
    let mut resp = Response::new();
    resp.set_mut(Template::new("template", data)).set_mut(status::Ok);
    Ok(resp)*/
}


// Share Database connection between workers
#[derive(Copy, Clone)]
pub struct SharedDatabaseConnection;
impl Key for SharedDatabaseConnection { type Value = rusqlite::Connection; }

// Share Configuration between workers
#[derive(Copy, Clone)]
pub struct SharedConfiguration;
impl Key for SharedConfiguration { type Value = ::Config; }

#[cfg(feature = "watch")]
pub fn get_handlebars_engine() -> impl AfterMiddleware {
    /// HandlebarsEngine will look up all files with "./examples/templates/**/*.hbs"
    let mut hbse = HandlebarsEngine::new();
    hbse.add(Box::new(DirectorySource::new("./templates/", ".hbs")));

    // load templates from all registered sources
    if let Err(r) = hbse.reload() {
        panic!("{}", r);
    }

    use std::sync::Arc;
    use handlebars_iron::Watchable;
    
    let hbse_ref = Arc::new(hbse);
    hbse_ref.watch("./templates/");
    hbse_ref
}

#[cfg(not(feature = "watch"))]
pub fn get_handlebars_engine() -> impl AfterMiddleware {
    /// HandlebarsEngine will look up all files with "./examples/templates/**/*.hbs"
    let mut hbse = HandlebarsEngine::new();
    hbse.add(Box::new(DirectorySource::new("./templates/", ".hbs")));

    // load templates from all registered sources
    if let Err(r) = hbse.reload() {
        panic!("{}", r);
    }

    hbse
}

fn cookie_warning(req: &mut Request) -> IronResult<Response> {
    match (get_session_token(req)) {
        Some(session_token) => {
            // TODO: Set session!
            // TODO:
            Ok(Response::with((status::Found, RedirectRaw(format!("/{}",req.url.query().unwrap_or(""))))))
        }, 
        None => {
            let mut resp = Response::new();
            resp.set_mut(Template::new("cookie", json_val::Map::new())).set_mut(status::Ok);
            Ok(resp)
        }
    }
    
        
}

pub fn start_server(conn: Connection, config: ::Config) {
    let router = router!(
        greet: get "/" => greet_personal,
        contests: get "/contest/" => contests,
        contest: get "/contest/:contestid" => contest,
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
        /*contest_load_par: get "/load" => task_par,
        contest_save_par: post "/save" => task_post_par,*/        
        task: get "/task/:taskid" => task,
        /*load: get "/load" => load_task,
        save: post "/save" => save_task,*/
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
    ch.link(Write::<SharedConfiguration>::both(config));
    ch.link_around(SessionStorage::new(SignedCookieBackend::new(my_secret)));

    ch.link_after(get_handlebars_engine());
    ch.link_after(ErrorReporter);
    
    let _res = Iron::new(ch).http("[::]:8080");
    println!("Listening on 8080.");
}


