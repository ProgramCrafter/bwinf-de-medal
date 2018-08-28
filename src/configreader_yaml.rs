use db_objects::{Contest, Taskgroup, Task};

use serde_yaml;

#[derive(Debug, Deserialize)]
struct ContestYaml {
    name: Option<String>,
    participation_start: Option<String>,
    participation_end: Option<String>,
    duration_minutes: Option<u32>,
    public_listing: Option<bool>,
    
    tasks: Option<serde_yaml::Mapping>,
}



pub fn parse_yaml(content: &str, filename: &str, directory: &str) -> Option<Contest> {
    let config: ContestYaml = serde_yaml::from_str(&content).unwrap();

    println!("hi");
    
    let mut contest = Contest::new(directory.to_string(), filename.to_string(), config.name?, config.duration_minutes?, config.public_listing.unwrap_or(false), None, None);

    println!("hi");
    
    for (name, info) in config.tasks? {
        println!("hi");
        if let serde_yaml::Value::String(name) = name {
            let mut taskgroup = Taskgroup::new(name);
            match info {
                serde_yaml::Value::String(taskdir) => {
                    let mut task = Task::new(taskdir, 3);
                    taskgroup.tasks.push(task);
                },
                serde_yaml::Value::Sequence(taskdirs) => {
                    let mut stars = 2;
                    for taskdir in taskdirs {
                        if let serde_yaml::Value::String(taskdir) = taskdir {
                            let mut task = Task::new(taskdir, stars);
                            taskgroup.tasks.push(task);
                        }
                        else {
                            panic!("Invalid contest YAML: {}{} (a)", directory, filename)
                        }
                        
                        stars += 1;
                    }
                }
                serde_yaml::Value::Mapping(taskdirs) => {
                    let mut stars = 2;
                    for (taskdir, taskinfo) in taskdirs {
                        if let (serde_yaml::Value::String(taskdir), serde_yaml::Value::Mapping(taskinfo)) = (taskdir, taskinfo) {
                            if let Some(serde_yaml::Value::Number(cstars)) = taskinfo.get(&serde_yaml::Value::String("stars".to_string())) {
                                stars = cstars.as_u64().unwrap() as u8;
                            }
                            let mut task = Task::new(taskdir, stars);
                            taskgroup.tasks.push(task);
                            stars += 1;
                        }
                        else {
                            panic!("Invalid contest YAML: {}{} (b)", directory, filename)
                        }
                    }
                }
                _ => panic!("Invalid contest YAML: {}{} (c)", directory, filename)
            }
            contest.taskgroups.push(taskgroup);
        }
        else {
            panic!("Invalid contest YAML: {}{} (d)", directory, filename)
        }
    }

    Some(contest)
}

