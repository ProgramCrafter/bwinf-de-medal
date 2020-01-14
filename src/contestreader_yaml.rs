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

    tasks: Option<serde_yaml::Mapping>,
}

// The task path is stored relatively to the contest.yaml for easier identificationy
// Concatenation happens in functions::show_task
pub fn parse_yaml(content: &str, filename: &str, directory: &str) -> Option<Contest> {
    let config: ContestYaml = serde_yaml::from_str(&content).unwrap();

    use self::time::{strptime, Timespec};
    
    let mut contest = Contest::new(directory.to_string(),
                                   filename.to_string(),
                                   config.name?,
                                   config.duration_minutes?,
                                   config.public_listing.unwrap_or(false),
                                   config.participation_start.map(|x| strptime(&x, &"%FT%T%z").map(|t| t.to_timespec()).unwrap_or_else(|_| Timespec::new(0, 0))),
                                   config.participation_end.map(|x| strptime(&x, &"%FT%T%z").map(|t| t.to_timespec()).unwrap_or_else(|_| Timespec::new(0, 0))));
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
