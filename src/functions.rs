use webfw_iron::{to_json, json_val};

use rusqlite::Connection;

use db_conn::{MedalConnection, MedalObject};

use db_objects::{Submission, Group};

pub fn blaa() -> (String, json_val::Map<String, json_val::Value>) {
    let mut data = json_val::Map::new();

    let mut contests = Vec::new();
    contests.push("blaa".to_string());
    data.insert("contest".to_string(), to_json(&contests));

    ("greeting".to_owned(), data)
}



#[derive(Serialize, Deserialize)]
pub struct SubTaskInfo {
    pub id: u32,
    pub linktext: String,
}

#[derive(Serialize, Deserialize)]
pub struct TaskInfo {
    pub name: String,
    pub subtasks: Vec<SubTaskInfo>,
}

#[derive(Serialize, Deserialize)]
pub struct ContestInfo {
    pub id: u32,
    pub location: String,
    pub filename: String,
    pub name: String,
    pub duration: u32,
    pub public: bool,
    pub tasks: Vec<TaskInfo>,
}


pub fn show_contests<T: MedalConnection>(conn: &T) -> (String, json_val::Map<String, json_val::Value>) {
    let mut data = json_val::Map::new();

    let v : Vec<ContestInfo> = conn.get_contest_list().iter().map(|c| { ContestInfo {
        id: c.id.unwrap(),
        location: c.location.clone(),
        filename: c.filename.clone(),
        name: c.name.clone(),
        duration: c.duration,
        public: c.public,
        tasks: Vec::new(),
    }}).collect();
    data.insert("contest".to_string(), to_json(&v));

    ("contests".to_owned(), data)
}


pub fn show_contest<T: MedalConnection>(conn: &T, contest_id: u32, session_token: String) -> (String, json_val::Map<String, json_val::Value>) {
    use std;
    let c = conn.get_contest_by_id_complete(contest_id);

    let mut tasks = Vec::new();
    for task in c.taskgroups {
        let mut stasks = Vec::new();
        for st in task.tasks {
            stasks.push(SubTaskInfo{id: st.id.unwrap(), linktext: str::repeat("☆", st.stars as usize)})
        }
        let mut ti = TaskInfo {name: task.name,
                               subtasks: stasks};
        tasks.push(ti);
    }
    
    let ci = ContestInfo {
        id: c.id.unwrap(),
        location: c.location.clone(),
        filename: c.filename.clone(),
        name: c.name.clone(),
        duration: c.duration,
        public: c.public,
        tasks: tasks,
    };

    let mut data = json_val::Map::new();
    data.insert("contest".to_string(), to_json(&ci));

    match conn.get_participation(session_token, contest_id) {
        None => {
            ("contest".to_owned(), data)
        },
        Some(participation) => {
            let now = time::get_time();
            let passed_secs = now.sec - participation.start.sec;
            if passed_secs < 0 {
                // behandle inkonsistente Serverzeit
            }

            data.insert("participation_start_date".to_string(), to_json(&format!("{}",passed_secs)));
            
            let left_secs = (ci.duration as i64) * 60 - passed_secs;
            if left_secs < 0 {
                // Contest over
                                
                    
            }
            else {
                let left_min = left_secs / 60;
                let left_sec = left_secs % 60;
                if left_sec < 10 {                    
                    data.insert("time_left".to_string(), to_json(&format!("{}:0{}",left_min,left_sec)));
                }
                else {
                    data.insert("time_left".to_string(), to_json(&format!("{}:{}",left_min,left_sec)));
                }
            }
            
            ("contest".to_owned(), data)
        }
    }
}

pub fn start_contest<T: MedalConnection>(conn: &T, contest_id: u32, session_token: String, csrf_token:String) -> Result<(),(String, json_val::Map<String, json_val::Value>)> {
    let mut data = json_val::Map::new();

    match conn.new_participation(session_token, contest_id) {
        Ok(_) => Ok(()),
        _ => Err(("conteststartfail".to_owned(), data))
    }
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

pub fn login_with_code<T: MedalConnection>(conn: &T, code: String) -> Result<Result<String, String>, (String, json_val::Map<String, json_val::Value>)> {
    match conn.login_with_code(None, code.clone()) {
        Ok(session_token) => {
            Ok(Ok(session_token))
        },
        Err(()) => {
            match conn.create_user_with_groupcode(None, code.clone()) {
                Ok(session_token) => {
                    Ok(Err(session_token))
                },
                Err(()) => {
                    let mut data = json_val::Map::new();
                    data.insert("reason".to_string(), to_json(&"Kein gültiger Code".to_string()));
                    data.insert("code".to_string(), to_json(&code));
                    Err(("login".to_owned(), data))   
                }
            }
        }
    }
}


pub fn logout<T: MedalConnection>(conn: &T, session_token: Option<String>) -> () {
    match session_token {
        Some(token) => conn.logout(token), _ => ()
    }
}


pub fn load_submission<T: MedalConnection>(conn: &T, task_id: u32, session_token: String) -> Result<String,()> {
    let session = conn.get_session(session_token).unwrap(); // TODO handle error
    
    match conn.load_submission(&session, task_id, None) {
        Some(submission) => Ok(submission.value),
        None => Ok("{}".to_string())
    }
}

pub fn save_submission<T: MedalConnection>(conn: &T, task_id: u32, session_token: String, csrf_token: String, data: String) -> Result<String,()> {
    let session = conn.get_session(session_token).unwrap(); // TODO handle error

    if session.csrf_token != csrf_token {
        return Err(());
    }

    let submission = Submission {
        id: None,
        session_user: session.id,
        task: task_id,
        grade: 0,
        validated: false,
        nonvalidated_grade: 0,
        needs_validation: true,
        subtask_identifier: None,
        value: data,
        date: time::get_time()
    };
    
    conn.submit_submission(submission);

    Ok("{}".to_string())
}


pub fn show_task<T: MedalConnection>(conn: &T, task_id: u32, session_token: String) -> (String, json_val::Map<String, json_val::Value>) {
    let session = conn.get_session(session_token).unwrap(); // TODO handle error

    let (t, tg, c) = conn.get_task_by_id_complete(task_id);

    let taskpath = format!("{}{}", c.location, t.location);
    
    let mut data = json_val::Map::new();

    data.insert("name".to_string(), to_json(&tg.name));
    data.insert("taskid".to_string(), to_json(&task_id));
    data.insert("csrftoken".to_string(), to_json(&session.csrf_token));
    data.insert("taskpath".to_string(), to_json(&taskpath));

    ("task".to_owned(), data)
        
}
//?state=42&scope=authenticate&code=250a4f49-e122-4b10-8da0-bc400ba5ea3d
// TOKEN  ->  {"token_type" : "Bearer","expires" : 3600,"refresh_token" : "R3a716e23-b320-4dab-a529-4c19e6b7ffc5","access_token" : "A6f681904-ded6-4e8b-840e-ac79ca1ffc07"}
// DATA  ->  {"lastName" : "Czechowski","gender" : "?","userType" : "a","userID" : "12622","dateOfBirth" : "2001-01-01","firstName" : "Robert","eMail" : "czechowski@bwinf.de","schoolId" : -1}

#[derive(Serialize, Deserialize)]
pub struct GroupInfo {
    pub id: u32, 
    pub name: String,
    pub tag: String,
    pub code: String,
}

pub fn show_groups<T: MedalConnection>(conn: &T, session_token: String) ->  (String, json_val::Map<String, json_val::Value>) {
    let session = conn.get_session(session_token).unwrap(); // TODO handle error

//    let groupvec = conn.get_group(session_token);

    let mut data = json_val::Map::new();

    let v : Vec<GroupInfo> = conn.get_groups(session.id).iter().map(|g| { GroupInfo {
        id: g.id.unwrap(),
        name: g.name.clone(),
        tag: g.tag.clone(),
        code: g.groupcode.clone(),
    }}).collect();
    data.insert("group".to_string(), to_json(&v));
    data.insert("csrftoken".to_string(), to_json(&session.csrf_token));
    
    ("groups".to_string(), data)
}

#[derive(Serialize, Deserialize)]
pub struct MemberInfo {
    pub id: u32, 
    pub firstname: String,
    pub lastname: String,
    pub grade: u8,
    pub logincode: String,
}

pub fn show_group<T: MedalConnection>(conn: &T, group_id: u32, session_token: String) -> Result<(String, json_val::Map<String, json_val::Value>),(String, json_val::Map<String, json_val::Value>)> {
    let session = conn.get_session(session_token).unwrap(); // TODO handle error
    let group = conn.get_group_complete(group_id).unwrap(); // TODO handle error

    let mut data = json_val::Map::new();
    
    if group.admin != session.id {
        return Err(("error".to_owned(), data));
    }

    let gi = GroupInfo {
        id: group.id.unwrap(),
        name: group.name.clone(),
        tag: group.tag.clone(),
        code: group.groupcode.clone(),
    };

    let v : Vec<MemberInfo> = group.members.iter().map(|m| { MemberInfo {
        id: m.id,
        firstname: m.firstname.clone().unwrap_or("".to_string()),
        lastname: m.lastname.clone().unwrap_or("".to_string()),
        grade: m.grade,
        logincode: m.logincode.clone().unwrap_or("".to_string()),
    }}).collect();

    data.insert("group".to_string(), to_json(&gi));
    data.insert("member".to_string(), to_json(&v));

    Ok(("group".to_string(), data))
}

pub fn modify_group<T: MedalConnection>(conn: &T, group_id: u32, session_token: String) -> Result<(),(String, json_val::Map<String, json_val::Value>)> {
    unimplemented!()
}

pub fn add_group<T: MedalConnection>(conn: &T, session_token: String, csrf_token: String, name: String, tag: String) -> Result<u32, (String, json_val::Map<String, json_val::Value>)> {
    let session = conn.get_session(session_token).unwrap(); // TODO handle error

    if session.csrf_token != csrf_token {
        let mut data = json_val::Map::new();
        return Err(("error".to_owned(), data));
    }

    let mut group = Group {
        id: None,
        name: name,
        groupcode: "blub".to_string(),
        tag: tag,
        admin: session.id,
        members: Vec::new()
    };

    conn.add_group(&mut group);

    Ok(group.id.unwrap())
}
    

pub fn show_profile<T: MedalConnection>(conn: &T, session_token: String) -> (String, json_val::Map<String, json_val::Value>) {
    let session = conn.get_session(session_token).unwrap(); // TODO handle error

    let mut data = json_val::Map::new();
    
    data.insert("firstname".to_string(), to_json(&session.firstname));
    data.insert("lastname".to_string(), to_json(&session.lastname));
    data.insert("csrftoken".to_string(), to_json(&session.csrf_token));
    data.insert("username".to_string(), to_json(&session.username));
    data.insert(format!("sel{}", session.grade), to_json(&"selected"));

    ("profile".to_string(), data)
}


pub fn edit_profile<T: MedalConnection>(conn: &T, session_token: String, csrf_token: String, firstname: String, lastname: String, grade: u8) -> Result<(),(String, json_val::Map<String, json_val::Value>)> {
    let mut session = conn.get_session(session_token).unwrap(); // TODO handle error

    if session.csrf_token != csrf_token {
        let mut data = json_val::Map::new();
        return Err(("error".to_owned(), data));
    }

    session.firstname = Some(firstname);
    session.lastname = Some(lastname);
    session.grade = grade;

    conn.save_session(session);

    Ok(())
}

#[derive(PartialEq)]
pub enum UserType {
    User,
    Teacher,
    Admin,
}

pub enum UserGender {
    Female,
    Male,
    Unknown,
}

pub struct ForeignUserData {
    pub foreign_id:   u32,
    pub foreign_type: UserType,
    pub gender:       UserGender,
    pub firstname:    String,
    pub lastname:     String,
}


pub fn login_oauth<T: MedalConnection>(conn: &T, user_data: ForeignUserData) -> Result<String, (String, json_val::Map<String, json_val::Value>)> {
    match conn.login_foreign(None, user_data.foreign_id, user_data.foreign_type, user_data.firstname, user_data.lastname) {
        Ok(session_token) => {
            Ok(session_token)
        },
        Err(()) => {
            let mut data = json_val::Map::new();
            data.insert("reason".to_string(), to_json(&"OAuth-Login failed.".to_string()));
            Err(("login".to_owned(), data))   
        }
    }

}
