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

use db_objects::{Contest, Task, Taskgroup};

use serde_yaml;
use std::path::Path;

extern crate time;

#[derive(Debug, Deserialize)]
struct ContestYaml {
    name: Option<String>,
    participation_start: Option<String>,
    participation_end: Option<String>,
    duration_minutes: Option<i32>,
    public_listing: Option<bool>,

    requires_login: Option<bool>,
    requires_contest: Option<Vec<String>>,
    secret: Option<String>,
    message: Option<String>,

    min_grade: Option<i32>,
    max_grade: Option<i32>,
    position: Option<i32>,

    tasks: Option<serde_yaml::Mapping>,
}

// The task path is stored relatively to the contest.yaml for easier identificationy
// Concatenation happens in functions::show_task
fn parse_yaml(content: &str, filename: &str, directory: &str) -> Option<Contest> {
    let config: ContestYaml = match serde_yaml::from_str(&content) {
        Ok(contest) => contest,
        Err(e) => {
            eprintln!();
            eprintln!("{}", e);
            eprintln!("Error loading contest YAML: {}{}", directory, filename);
            panic!("Loading contest file")
        }
    };

    use self::time::{strptime, Timespec};

    let mut contest =
        Contest::new(directory.to_string(),
                     filename.to_string(),
                     config.name?,
                     config.duration_minutes?,
                     config.public_listing.unwrap_or(false),
                     config.participation_start
                           .map(|x| {
                               strptime(&x, &"%FT%T%z").map(|t| t.to_timespec()).unwrap_or_else(|_| Timespec::new(0, 0))
                           }),
                     config.participation_end
                           .map(|x| {
                               strptime(&x, &"%FT%T%z").map(|t| t.to_timespec()).unwrap_or_else(|_| Timespec::new(0, 0))
                           }),
                     config.min_grade,
                     config.max_grade,
                     config.position,
                     config.requires_login,
                     // Consumed by `let required_contests = contest.requires_contest.as_ref()?.split(',');` in core.rs
                     config.requires_contest.map(|list| list.join(",")),
                     config.secret,
                     config.message);
    // TODO: Timeparsing should fail more pleasantly (-> Panic, thus shows message)

    for (positionalnumber, (name, info)) in config.tasks?.into_iter().enumerate() {
        if let serde_yaml::Value::String(name) = name {
            let mut taskgroup = Taskgroup::new(name, Some(positionalnumber as i32));
            match info {
                serde_yaml::Value::String(taskdir) => {
                    let task = Task::new(taskdir, 3);
                    taskgroup.tasks.push(task);
                }
                serde_yaml::Value::Sequence(taskdirs) => {
                    let mut stars = 2;
                    for taskdir in taskdirs {
                        if let serde_yaml::Value::String(taskdir) = taskdir {
                            let task = Task::new(taskdir, stars);
                            taskgroup.tasks.push(task);
                        } else {
                            panic!("Invalid contest YAML: {}{} (a)", directory, filename)
                        }

                        stars += 1;
                    }
                }
                serde_yaml::Value::Mapping(taskdirs) => {
                    let mut stars = 2;
                    for (taskdir, taskinfo) in taskdirs {
                        if let (serde_yaml::Value::String(taskdir), serde_yaml::Value::Mapping(taskinfo)) =
                            (taskdir, taskinfo)
                        {
                            if let Some(serde_yaml::Value::Number(cstars)) =
                                taskinfo.get(&serde_yaml::Value::String("stars".to_string()))
                            {
                                stars = cstars.as_u64().unwrap() as i32;
                            }
                            let task = Task::new(taskdir, stars);
                            taskgroup.tasks.push(task);
                            stars += 1;
                        } else {
                            panic!("Invalid contest YAML: {}{} (b)", directory, filename)
                        }
                    }
                }
                _ => panic!("Invalid contest YAML: {}{} (c)", directory, filename),
            }
            contest.taskgroups.push(taskgroup);
        } else {
            panic!("Invalid contest YAML: {}{} (d)", directory, filename)
        }
    }

    Some(contest)
}

fn read_contest(p: &Path) -> Option<Contest> {
    use std::fs::File;
    use std::io::Read;

    let mut file = File::open(p).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).ok()?;

    parse_yaml(&contents, p.file_name().to_owned()?.to_str()?, &format!("{}/", p.parent().unwrap().to_str()?))
}

pub fn get_all_contest_info(task_dir: &str) -> Vec<Contest> {
    fn walk_me_recursively(p: &Path, contests: &mut Vec<Contest>) {
        if let Ok(paths) = std::fs::read_dir(p) {
            print!("…");
            use std::io::Write;
            std::io::stdout().flush().unwrap();
            let mut paths: Vec<_> = paths.filter_map(|r| r.ok()).collect();
            paths.sort_by_key(|dir| dir.path());
            for path in paths {
                let p = path.path();
                walk_me_recursively(&p, contests);
            }
        }

        if p.file_name().unwrap().to_string_lossy().to_string().ends_with(".yaml") {
            read_contest(p).map(|contest| contests.push(contest));
        };
    }

    let mut contests = Vec::new();
    match std::fs::read_dir(task_dir) {
        Err(why) => println!("Error opening tasks directory! {:?}", why.kind()),
        Ok(paths) => {
            for path in paths {
                walk_me_recursively(&path.unwrap().path(), &mut contests);
            }
        }
    };

    contests
}
