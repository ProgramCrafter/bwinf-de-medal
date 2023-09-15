/*  medal                                                                                                            *\
 *  Copyright (C) 2022  Bundesweite Informatikwettbewerbe, Robert Czechowski                                                            *
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
    review_start: Option<String>,
    review_end: Option<String>,
    duration_minutes: Option<i32>,
    public_listing: Option<bool>,

    protected: Option<bool>,
    requires_login: Option<bool>,
    requires_contest: Option<Vec<String>>,
    secret: Option<String>,
    message: Option<String>,
    image: Option<String>,
    language: Option<String>,
    category: Option<String>,

    min_grade: Option<i32>,
    max_grade: Option<i32>,
    position: Option<i32>,

    tasks: Option<serde_yaml::Mapping>,
}

#[derive(Debug, Deserialize)]
struct TaskYaml {
    name: Option<String>,
    standalone: Option<bool>,
    public_listing: Option<bool>,

    position: Option<i32>,

    languages: Option<Vec<String>>,
}

use self::time::{strptime, Timespec};

fn parse_timespec(time: String, key: &str, directory: &str, filename: &str) -> Timespec {
    strptime(&time, &"%FT%T%z").map(|t| t.to_timespec())
                               .unwrap_or_else(|_| {
                                   panic!("Time value '{}' could not be parsed in {}{}", key, directory, filename)
                               })
}

// The task path is stored relatively to the contest.yaml for easier identificationy
// Concatenation happens in functions::show_task
fn parse_contest_yaml(content: &str, filename: &str, directory: &str) -> Option<Vec<Contest>> {
    let config: ContestYaml = match serde_yaml::from_str(&content) {
        Ok(contest) => contest,
        Err(e) => {
            eprintln!();
            eprintln!("{}", e);
            eprintln!("Error loading contest YAML: {}{}", directory, filename);
            panic!("Loading contest file")
        }
    };

    let start: Option<Timespec> =
        config.participation_start.map(|x| parse_timespec(x, "participation_start", directory, filename));
    let end: Option<Timespec> =
        config.participation_end.map(|x| parse_timespec(x, "participation_end", directory, filename));
    let review_start: Option<Timespec> =
        config.review_start.map(|x| parse_timespec(x, "review_start", directory, filename));
    let review_end: Option<Timespec> = config.review_end.map(|x| parse_timespec(x, "review_end", directory, filename));

    let review_start = if review_end.is_none() {
        review_start
    } else if let Some(end) = end {
        Some(review_start.unwrap_or(end))
    } else {
        review_start
    };

    let mut contest =
        Contest { id: None,
                  location: directory.to_string(),
                  filename: filename.to_string(),
                  name: config.name.unwrap_or_else(|| panic!("'name' missing in {}{}", directory, filename)),
                  duration:
                      config.duration_minutes
                            .unwrap_or_else(|| panic!("'duration_minutes' missing in {}{}", directory, filename)),
                  public: config.public_listing.unwrap_or(false),
                  start,
                  end,
                  review_start,
                  review_end,
                  min_grade: config.min_grade,
                  max_grade: config.max_grade,
                  positionalnumber: config.position,
                  protected: config.protected.unwrap_or(false),
                  requires_login: config.requires_login,
                  // Consumed by `let required_contests = contest.requires_contest.as_ref()?.split(',');` in core.rs
                  requires_contest: config.requires_contest.map(|list| list.join(",")),
                  secret: config.secret,
                  message: config.message,
                  image: config.image,
                  language: config.language,
                  category: config.category,
                  standalone_task: None,
                  taskgroups: Vec::new() };
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

    Some(vec![contest])
}

#[derive(Debug)]
enum ConfigError {
    ParseError(serde_yaml::Error),
    MissingField,
}

fn parse_task_yaml(content: &str, filename: &str, directory: &str) -> Result<Vec<Contest>, ConfigError> {
    let config: TaskYaml = serde_yaml::from_str(&content).map_err(ConfigError::ParseError)?;

    // Only handle tasks with standalone = true
    if config.standalone != Some(true) {
        return Err(ConfigError::MissingField);
    }

    // Currently only tasks with a single language are supported
    let language = if let Some(languages) = config.languages {
        if languages.len() != 1 {
            return Err(ConfigError::MissingField);
        } else {
            languages[0].clone()
        }
    } else {
        return Err(ConfigError::MissingField);
    };

    let name = config.name.unwrap_or_else(|| panic!("'name' missing in {}{}", directory, filename));
    let mut contest = Contest { id: None,
                                location: directory.to_string(),
                                filename: filename.to_string(),
                                name: name.clone(),
                                // Task always are unlimited in time
                                duration: 0,
                                public: config.public_listing.unwrap_or(false),
                                start: None,
                                end: None,
                                review_start: None,
                                review_end: None,
                                min_grade: None,
                                max_grade: None,
                                positionalnumber: config.position,
                                protected: false,
                                requires_login: Some(false),
                                // Consumed by `let required_contests = contest.requires_contest.as_ref()?.split(',');` in core.rs
                                requires_contest: None,
                                secret: None,
                                message: None,
                                image: None,
                                language: Some(language.clone()),
                                category: None,
                                standalone_task: Some(true),
                                taskgroups: Vec::new() };

    let mut taskgroup = Taskgroup::new(name, None);
    let stars = 0;
    let taskdir = if language == "blockly" {
        "B.".to_string()
    } else if language == "python" {
        "P.".to_string()
    } else {
        ".".to_string()
    };
    let task = Task::new(taskdir, stars);
    taskgroup.tasks.push(task);
    contest.taskgroups.push(taskgroup);

    Ok(vec![contest])

    // Err(ConfigError::MissingField)
}

fn read_task_or_contest(p: &Path) -> Option<Vec<Contest>> {
    use std::fs::File;
    use std::io::Read;

    let mut file = File::open(p).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).ok()?;

    let filename: &str = p.file_name().to_owned()?.to_str()?;

    if filename == "task.yaml" {
        parse_task_yaml(&contents, filename, &format!("{}/", p.parent().unwrap().to_str()?)).ok()
    } else {
        parse_contest_yaml(&contents, filename, &format!("{}/", p.parent().unwrap().to_str()?))
    }
}

pub fn get_all_contest_info(task_dir: &str) -> Vec<Contest> {
    fn walk_me_recursively(p: &Path, contests: &mut Vec<Contest>) {
        if let Ok(paths) = std::fs::read_dir(p) {
            {
                use std::io::Write;
                print!(".");
                std::io::stdout().flush().unwrap();
            }
            let mut paths: Vec<_> = paths.filter_map(|r| r.ok()).collect();
            paths.sort_by_key(|dir| dir.path());
            for path in paths {
                let p = path.path();
                walk_me_recursively(&p, contests);
            }
        }

        let filename = p.file_name().unwrap().to_string_lossy().to_string();
        if filename.ends_with(".yaml") {
            read_task_or_contest(p).as_mut().map(|cs| contests.append(cs));
        };
    }

    let mut contests = Vec::new();
    match std::fs::read_dir(task_dir) {
        Err(why) => eprintln!("Error opening tasks directory! {:?}", why.kind()),
        Ok(paths) => {
            for path in paths {
                walk_me_recursively(&path.unwrap().path(), &mut contests);
            }
        }
    };

    contests
}

#[test]
fn parse_contest_yaml_no_tasks() {
    let contest_file_contents = r#"
name: "JwInf 2020 Runde 1: Jgst. 3 – 6"
duration_minutes: 60
"#;

    let contest = parse_contest_yaml(contest_file_contents, "", "");
    assert!(contest.is_none());
}

#[test]
fn parse_contest_yaml_dates() {
    let contest_file_contents = r#"
name: "JwInf 2020 Runde 1: Jgst. 3 – 6"
participation_start: "2022-03-01T00:00:00+01:00"
participation_end: "2022-03-31T22:59:59+01:00"
duration_minutes: 60

tasks: {}
"#;

    let contest = parse_contest_yaml(contest_file_contents, "", "");
    assert!(contest.is_some());

    //let contest = contest.unwrap();

    // These tests are unfortunately dependent on the timezone the system is on. Skip them for now until we have found
    // a better solution.

    //assert_eq!(contest.start, Some(Timespec {sec: 1646089200, nsec: 0}));
    //assert_eq!(contest.end, Some(Timespec {sec: 1648763999, nsec: 0}));

    // Unix Timestamp 	1646089200
    // GMT 	Mon Feb 28 2022 23:00:00 GMT+0000
    // Your Time Zone 	Tue Mar 01 2022 00:00:00 GMT+0100 (Mitteleuropäische Normalzeit)

    // Unix Timestamp 	1648764000
    // GMT 	Thu Mar 31 2022 22:00:00 GMT+0000
    // Your Time Zone 	Fri Apr 01 2022 00:00:00 GMT+0200 (Mitteleuropäische Sommerzeit)
}
