extern crate rusqlite;

use rusqlite::Connection;

mod db_apply_migrations;
mod db_conn_sqlite;
mod db_conn;
mod db_objects;

use db_conn::{MedalConnection, MedalObject};

use db_objects::*;

fn main() {
    let mut conn = Connection::create();
    db_apply_migrations::test(&mut conn);

    let mut contest = Contest::new("./".to_string(), "blub.json".to_string(), "Wettbewerb IX".to_string(), 45, true, None, None);
    let mut taskgroup = Taskgroup::new("Lustige Aufgabe".to_string());
    let mut task = Task::new("blub".to_string(), 1);
    taskgroup.tasks.push(task);
    let mut task = Task::new("blub2".to_string(), 4);
    taskgroup.tasks.push(task);  
    contest.taskgroups.push(taskgroup);
    let mut taskgroup = Taskgroup::new("Lustige Aufgabe3".to_string());
    let mut task = Task::new("blub3".to_string(), 2);
    taskgroup.tasks.push(task);
    let mut task = Task::new("blub4".to_string(), 3);
    taskgroup.tasks.push(task);  
    contest.taskgroups.push(taskgroup);
    contest.save(&mut conn);
        
    println!("Hello, world!");

    let contest = conn.get_contest_by_id_complete(1);

    println!("Contest {}", contest.name);
    
    for taskgroup in contest.taskgroups {
        print!("  Task {}: ", taskgroup.name);
        for task in taskgroup.tasks {
            print!("{} ({}) ", task.stars, task.location);
        }
        println!("");
    }
}
