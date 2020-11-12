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
pub fn parse_yaml(content: &str, filename: &str, directory: &str) -> Option<Contest> {
    let config: ContestYaml = serde_yaml::from_str(&content).unwrap();

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
