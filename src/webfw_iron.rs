
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

    let (template, data) = {
        // hier ggf. Daten aus dem Request holen
        let mutex = req.get::<Write<SharedDatabaseConnection>>().unwrap();
        let conn = mutex.lock().unwrap();

        // Antwort erstellen und zurücksenden   
        functions::show_contest(&*conn, contest_id)
    };

    let mut resp = Response::new();
    resp.set_mut(Template::new(&template, data)).set_mut(status::Ok);
    Ok(resp)
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


// Share Database connection between workers
#[derive(Copy, Clone)]
pub struct SharedDatabaseConnection;
impl Key for SharedDatabaseConnection { type Value = rusqlite::Connection; }

pub fn start_server(conn: Connection) {
    let router = router!(
        greet: get "/" => greet,
        contests: get "/contest" => contests,
        contest: get "/contest/:contestid" => contest,
        login: get "/login" => login,
        login_post: post "/login" => login_post,
        logout: post "/logout" => logout,/*
        task: get "/task/:taskid" => show_task,
        load: get "/load" => load_task,
        save: post "/save" => save_task,*/
    );

    let my_secret = b"verysecret".to_vec();

    let mut mount = Mount::new();

    // Serve the shared JS/CSS at /
    mount.mount("/static/", Static::new(Path::new("static")));
    //mount.mount("/tasks/", Static::new(Path::new(TASK_DIR)));
    mount.mount("/", router);

    let mut ch = Chain::new(mount);
    
    ch.link(Write::<SharedDatabaseConnection>::both(conn));
    ch.link_around(SessionStorage::new(SignedCookieBackend::new(my_secret)));

    /// HandlebarsEngine will look up all files with "./examples/templates/**/*.hbs"
    let mut hbse = HandlebarsEngine::new();
    hbse.add(Box::new(DirectorySource::new("./templates/", ".hbs")));

    // load templates from all registered sources
    if let Err(r) = hbse.reload() {
        panic!("{}", r);
    }

    ch.link_after(hbse);
    ch.link_after(ErrorReporter);
    
    let _res = Iron::new(ch).http("[::]:8080");
    println!("Listening on 8080.");
}
