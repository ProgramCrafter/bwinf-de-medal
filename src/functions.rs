use webfw_iron::{to_json, json_val};

use rusqlite::Connection;

use db_conn::MedalConnection;

pub fn blaa() -> (String, json_val::Map<String, json_val::Value>) {
    let mut data = json_val::Map::new();

    let mut contests = Vec::new();
    contests.push("blaa".to_string());
    data.insert("contest".to_string(), to_json(&contests));

    ("greeting".to_owned(), data)
}



#[derive(Serialize, Deserialize)]
pub struct ContestInfo {
    pub id: u32,
    pub location: String,
    pub filename: String,
    pub name: String,
    pub duration: u32,
    pub public: bool,
}


pub fn show_contests<T: MedalConnection>(conn: &T) -> (String, json_val::Map<String, json_val::Value>) {
    let mut data = json_val::Map::new();
   

    let v : Vec<ContestInfo> = conn.get_contest_list().iter().map(|c| { ContestInfo {
        id: c.id.unwrap(),
        location: c.location.clone(),
        filename: c.filename.clone(),
        name: c.name.clone(),
        duration: c.duration,
        public: c.public
    }}).collect();
    data.insert("contest".to_string(), to_json(&v));

    ("contests".to_owned(), data)
}


pub fn show_contest<T: MedalConnection>(conn: &T, contest_id: u32) -> (String, json_val::Map<String, json_val::Value>) {  
    let c = conn.get_contest_by_id(contest_id);
    let ci = ContestInfo {
        id: c.id.unwrap(),
        location: c.location.clone(),
        filename: c.filename.clone(),
        name: c.name.clone(),
        duration: c.duration,
        public: c.public
    };


    let mut data = json_val::Map::new();
    data.insert("contest".to_string(), to_json(&ci));

    ("contest".to_owned(), data)
}


pub fn login<T: MedalConnection>(conn: &T, login_data: (String, String)) -> Result<String, (String, json_val::Map<String, json_val::Value>)> {
    let (username, password) = login_data;

    match conn.login(None, username.clone(), password) {
        Ok(session_token) => {
            Ok(session_token)
        },
        Err(()) => {
            let mut data = json_val::Map::new();
            data.insert("reason".to_string(), to_json(&"Not implemented".to_string()));
            data.insert("username".to_string(), to_json(&username));
            Err(("login".to_owned(), data))   
        }
    }
}


pub fn logout<T: MedalConnection>(conn: &T, session_token: Option<String>) -> () {
    match session_token {
        Some(token) => conn.logout(token), _ => ()
    }
}

//?state=42&scope=authenticate&code=250a4f49-e122-4b10-8da0-bc400ba5ea3d
// TOKEN  ->  {"token_type" : "Bearer","expires" : 3600,"refresh_token" : "R3a716e23-b320-4dab-a529-4c19e6b7ffc5","access_token" : "A6f681904-ded6-4e8b-840e-ac79ca1ffc07"}
// DATA  ->  {"lastName" : "Czechowski","gender" : "?","userType" : "a","userID" : "12622","dateOfBirth" : "2001-01-01","firstName" : "Robert","eMail" : "czechowski@bwinf.de","schoolId" : -1}

