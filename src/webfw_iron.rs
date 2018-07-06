
//extern crate serde;



use std::path::Path;

use iron_sessionstorage::traits::*;

use iron::prelude::*;
use iron::{status, AfterMiddleware};
use iron::modifiers::Redirect;

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
    SessionRequestExt::session(req).get::<SessionToken>().unwrap();
    // hier ggf. Daten aus dem Request holen

    // Daten verarbeiten
    let (template, data) = functions::blaa();

    // Antwort erstellen und zurücksenden  
    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}

fn contests(req: &mut Request) -> IronResult<Response> {
    let (template, data) = {
        // hier ggf. Daten aus dem Request holen
        let mutex = req.get::<Write<SharedDatabaseConnection>>().unwrap();
        let conn = mutex.lock().unwrap();

        // Antwort erstellen und zurücksenden   
        functions::show_contests(&*conn)
    };
    
    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}

fn contest(req: &mut Request) -> IronResult<Response> {
    let contest_id = req.extensions.get::<Router>().unwrap().find("contestid").unwrap_or("").parse::<u32>().unwrap_or(0);
    let session_token = SessionRequestExt::session(req).get::<SessionToken>().unwrap().unwrap();

    let (template, data) = {
        // hier ggf. Daten aus dem Request holen
        let mutex = req.get::<Write<SharedDatabaseConnection>>().unwrap();
        let conn = mutex.lock().unwrap();

        // Antwort erstellen und zurücksenden   
        functions::show_contest(&*conn, contest_id,  session_token.token)
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
        
        let conn = mutex.lock().unwrap();

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
        let conn = mutex.lock().unwrap();

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


fn logout(req: &mut Request) -> IronResult<Response> {
    let session_token = SessionRequestExt::session(req).get::<SessionToken>().unwrap();

    {
        let mutex = req.get::<Write<SharedDatabaseConnection>>().unwrap();
        let conn = mutex.lock().unwrap();

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
        let conn = mutex.lock().unwrap();
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
        let conn = mutex.lock().unwrap();
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
    let task_id = req.extensions.get::<Router>().unwrap().find("contestid").unwrap_or("").parse::<u32>().unwrap_or(0);
    // TODO: Make work without session
    let session_token = SessionRequestExt::session(req).get::<SessionToken>().unwrap().unwrap();
    
    println!("{}",task_id);

    let (template, data) = {
        let mutex = req.get::<Write<SharedDatabaseConnection>>().unwrap();
        let conn = mutex.lock().unwrap();
        functions::show_task(&*conn, task_id, session_token.token)
    };

    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}

fn groups(req: &mut Request) -> IronResult<Response> {
    let (template, data) = {
        // hier ggf. Daten aus dem Request holen
        let mutex = req.get::<Write<SharedDatabaseConnection>>().unwrap();
        let conn = mutex.lock().unwrap();

        // Antwort erstellen und zurücksenden   
        //functions::show_contests(&*conn)
        let mut data = json_val::Map::new();
        data.insert("reason".to_string(), to_json(&"Not implemented".to_string()));
        ("groups", data)
    };
    
    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}

fn group(req: &mut Request) -> IronResult<Response> {
    let (template, data) = {
        // hier ggf. Daten aus dem Request holen
        let mutex = req.get::<Write<SharedDatabaseConnection>>().unwrap();
        let conn = mutex.lock().unwrap();

        // Antwort erstellen und zurücksenden   
        //functions::show_contests(&*conn)
        let mut data = json_val::Map::new();
        data.insert("reason".to_string(), to_json(&"Not implemented".to_string()));
        ("group", data)
    };
    
    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
}

fn group_post(req: &mut Request) -> IronResult<Response> {
    Ok(Response::with((status::Found, Redirect(url_for!(req, "group")))))
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
        let conn = mutex.lock().unwrap();
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
    let (template, data) = {
        // hier ggf. Daten aus dem Request holen
        let mutex = req.get::<Write<SharedDatabaseConnection>>().unwrap();
        let conn = mutex.lock().unwrap();

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


// Share Database connection between workers
#[derive(Copy, Clone)]
pub struct SharedDatabaseConnection;
impl Key for SharedDatabaseConnection { type Value = rusqlite::Connection; }

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
}


pub fn start_server(conn: Connection) {
    let router = router!(
        greet: get "/" => greet,
        contests: get "/contest/" => contests,
        contest: get "/contest/:contestid" => contest,
        contest_post: post "/contest/:contestid" => contest_post,
        login: get "/login" => login,
        login_post: post "/login" => login_post,
        logout: post "/logout" => logout,
        subm: get "/submission/:taskid" => submission,
        subm_post: post "/submission/:taskid" => submission_post,
        subm_load: get "/load/:taskid" => submission,
        subm_save: post "/save/:taskid" => submission_post,
        groups: get "/group/" => groups,
        groups: post "/group/" => new_group,
        group: get "/group/:groupid" => group,
        group_post: post "/group" => group_post,
        profile: get "/profile" => profile,
        /*contest_load_par: get "/load" => task_par,
        contest_save_par: post "/save" => task_post_par,*/        
        task: get "/task/:taskid" => task,
        /*load: get "/load" => load_task,
        save: post "/save" => save_task,*/
    );

    let my_secret = b"verysecret".to_vec();

    let mut mount = Mount::new();

    // Serve the shared JS/CSS at /
    mount.mount("/static/", Static::new(Path::new("static")));
    mount.mount("/tasks/", Static::new(Path::new(TASK_DIR)));
    mount.mount("/", router);

    let mut ch = Chain::new(mount);
    
    ch.link(Write::<SharedDatabaseConnection>::both(conn));
    ch.link_around(SessionStorage::new(SignedCookieBackend::new(my_secret)));

    ch.link_after(get_handlebars_engine());
    ch.link_after(ErrorReporter);
    
    let _res = Iron::new(ch).http("[::]:8080");
    println!("Listening on 8080.");
}


