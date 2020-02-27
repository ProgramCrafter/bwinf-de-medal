//use std::path::Path;

//use config::Config;
//use core;
use db_conn::MedalConnection;

use std::error::Error;
//use std::io;

use std::fs::File;
use std::io::BufReader;

#[derive(Debug)]
pub struct UserData {
    pub firstname: String,
    pub lastname: String,
    pub grade: i32,
    pub sex: i32,
    pub logincode: Option<String>,
    pub pmsid: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub street: Option<String>,
    pub zip: Option<String>,
    pub city: Option<String>,
    pub nation: Option<String>,
}

#[derive(Debug)]
pub struct ParticipationData {
    pub startdate: String,
    pub contesttype: i32,
    pub results: [i32; 10],
}

#[derive(Debug)]
pub struct GroupData {
    pub groupname: String,
    pub groupcode: Option<String>,
}
#[derive(Debug)]
pub struct TeacherData {
    pub firstname: String,
    pub lastname: String,
    pub pmsid: String,
}

#[derive(Debug)]
pub struct Info {
    pub user: UserData,
    pub part: ParticipationData,
    pub group: Option<GroupData>,
    pub teacher: Option<TeacherData>,
}

fn read_data(filename: &str) -> Result<Vec<Info>, Box<dyn Error>> {
    let f = File::open(filename)?;
    let reader = BufReader::new(f);

    // Build the CSV reader and iterate over each record.
    let mut rdr = csv::Reader::from_reader(reader);
    rdr.records()
       .map(|res| -> Result<Info, Box<dyn Error>> {
           let rec = res?;
           println!("{:?}", rec);
           Ok(Info { user: UserData { firstname: rec[8].to_owned(),
                                      lastname: rec[9].to_owned(),
                                      grade: {let g: i32 = rec[10].parse()?; if g == -1 {0} else if g == -2 {255} else {12-g}} ,
                                      sex: if &rec[11] == "Male" { 1 } else { 0 },
                                      street: if &rec[12] != "NULL" { Some(rec[12].to_owned()) } else { None },
                                      zip: if &rec[13] != "NULL" { Some(rec[13].to_owned()) } else { None },
                                      city: if &rec[14] != "NULL" { Some(rec[14].to_owned()) } else { None },
                                      nation: Some("".to_owned()), //if &rec[7] != "NULL" { Some(rec[7].to_owned()) } else { None },
                                      logincode: if &rec[7] != "NULL" { Some(rec[7].to_owned()) } else { None },
                                      pmsid: if &rec[5] != "NULL" { Some(rec[5].to_owned()) } else { None },
                                      username: if &rec[4] != "NULL" { Some(rec[4].to_owned()) } else { None },
                                      password: if &rec[6] != "NULL" { Some(rec[6].to_owned()) } else { None } },
                     part: ParticipationData { startdate: "".to_owned(), contesttype: 0, results: [0; 10] },
                     group: if &rec[16] != "NULL" {
                         Some(GroupData { groupname: rec[16].to_owned(),
                                          groupcode: if &rec[17] != "NULL" {
                                              Some(rec[17].to_owned())
                                          } else {
                                              None
                                          } })
                     } else {
                         None
                     },
                     teacher: if &rec[19] != "NULL" {
                         Some(TeacherData { firstname: rec[20].to_owned(),
                                            lastname: rec[21].to_owned(),
                                            pmsid: rec[19].to_owned() })
                     } else {
                         None
                     } })
       })
       .collect()
}

pub fn import_foreign_contest<C>(conn: &mut C, filename: &str)
    where C: MedalConnection + std::marker::Send + 'static {
    match read_data(filename) {
        Err(err) => println!("error reading data: {}", err),
        Ok(v) => {
            println!("{:?}", v);
            println!("{:?}", conn.import_foreign_data(v));
        }
    }
}
