use super::*;

use db_objects::{Contest, Task, Taskgroup};

use reqwest::StatusCode;
use std::path::Path;

fn addsimpleuser(conn: &mut rusqlite::Connection, username: String, password: String, is_t: bool, is_a: bool) {
    let mut test_user = conn.new_session("");
    test_user.username = Some(username);
    test_user.is_teacher = is_t;
    test_user.is_admin = Some(is_a);
    test_user.set_password(&password).expect("Set Password did not work correctly.");
    conn.save_session(test_user);
}

fn start_server_and_fn<P, F>(port: u16, p: P, f: F)
    where F: Fn(),
          P: Fn(&mut rusqlite::Connection) + std::marker::Send + 'static
{
    use std::sync::mpsc::channel;
    let (start_tx, start_rx) = channel();
    let (stop_tx, stop_rx) = channel();

    std::thread::spawn(move || {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        db_apply_migrations::test(&mut conn);

        p(&mut conn);

        // ID: 1, gets renamed
        let mut contest = Contest::new("directory".to_string(),
                                       "public.yaml".to_string(),
                                       "RenamedContestName".to_string(),
                                       1, // Time: 1 Minute
                                       true,
                                       None,
                                       None,
                                       None,
                                       None,
                                       None,
                                       None,
                                       None,
                                       false,
                                       None,
                                       None,
                                       None,
                                       None);
        contest.save(&conn);

        // ID: 1
        let mut contest = Contest::new("directory".to_string(),
                                       "public.yaml".to_string(),
                                       "PublicContestName".to_string(),
                                       1, // Time: 1 Minute
                                       true,
                                       None,
                                       None,
                                       None,
                                       None,
                                       None,
                                       None,
                                       None,
                                       false,
                                       None,
                                       None,
                                       None,
                                       None);
        let mut taskgroup = Taskgroup::new("TaskgroupName".to_string(), None);
        let task = Task::new("taskdir1".to_string(), 3); // ID: 1
        taskgroup.tasks.push(task);
        let task = Task::new("taskdir2".to_string(), 4); // ID: 2
        taskgroup.tasks.push(task);
        contest.taskgroups.push(taskgroup);
        contest.save(&conn);

        // ID: 2
        let mut contest = Contest::new("directory".to_string(),
                                       "private.yaml".to_string(),
                                       "PrivateContestName".to_string(),
                                       1, // Time: 1 Minute
                                       false,
                                       None,
                                       None,
                                       None,
                                       None,
                                       None,
                                       None,
                                       None,
                                       false,
                                       None,
                                       None,
                                       None,
                                       None);
        let mut taskgroup = Taskgroup::new("TaskgroupName".to_string(), None);
        let task = Task::new("taskdir1".to_string(), 3); // ID: 3
        taskgroup.tasks.push(task);
        let task = Task::new("taskdir2".to_string(), 4); // ID: 4
        taskgroup.tasks.push(task);
        contest.taskgroups.push(taskgroup);
        contest.save(&conn);

        // ID: 3
        let mut contest = Contest::new("directory".to_string(),
                                       "infinte.yaml".to_string(),
                                       "InfiniteContestName".to_string(),
                                       0,
                                       true,
                                       None,
                                       None,
                                       None,
                                       None,
                                       None,
                                       None,
                                       None,
                                       false,
                                       None,
                                       None,
                                       None,
                                       None);
        let mut taskgroup = Taskgroup::new("TaskgroupRenameName".to_string(), None);
        let task = Task::new("taskdir1".to_string(), 3); // ID: 5
        taskgroup.tasks.push(task);
        let task = Task::new("taskdir2".to_string(), 4); // ID: 6
        taskgroup.tasks.push(task);
        contest.taskgroups.push(taskgroup);
        contest.save(&conn);

        let mut taskgroup = Taskgroup::new("TaskgroupNewName".to_string(), None);
        let task = Task::new("taskdir1".to_string(), 3); // ID: 5
        taskgroup.tasks.push(task);
        let task = Task::new("taskdir2".to_string(), 4); // ID: 6
        taskgroup.tasks.push(task);
        contest.taskgroups.push(taskgroup);
        contest.save(&conn);

        // ID: 4
        let mut contest = Contest::new("directory".to_string(),
                                       "publicround2.yaml".to_string(),
                                       "PublicContestRoundTwoName".to_string(),
                                       1, // Time: 1 Minute
                                       true,
                                       None,
                                       None,
                                       None,
                                       None,
                                       None,
                                       None,
                                       None,
                                       false,
                                       None,
                                       Some("public.yaml".to_string()),
                                       None,
                                       None);
        let mut taskgroup = Taskgroup::new("TaskgroupName".to_string(), None);
        let task = Task::new("taskdir1".to_string(), 3); // ID: 7
        taskgroup.tasks.push(task);
        let task = Task::new("taskdir2".to_string(), 4); // ID: 8
        taskgroup.tasks.push(task);
        contest.taskgroups.push(taskgroup);
        contest.save(&conn);

        // Have the contest review start one day in the past
        let mut in_the_past = time::get_time();
        in_the_past.sec -= 60 * 60 * 24;

        // ID: 5
        let mut contest = Contest::new("directory".to_string(),
                                       "current_review.yaml".to_string(),
                                       "CurrentReviewContestName".to_string(),
                                       1, // Time: 1 Minute
                                       true,
                                       None,
                                       None,
                                       Some(in_the_past),
                                       None,
                                       None,
                                       None,
                                       None,
                                       false,
                                       None,
                                       None,
                                       None,
                                       None);
        let mut taskgroup = Taskgroup::new("TaskgroupName".to_string(), None);
        let task = Task::new("taskdir1".to_string(), 3); // ID: 1
        taskgroup.tasks.push(task);
        let task = Task::new("taskdir2".to_string(), 4); // ID: 2
        taskgroup.tasks.push(task);
        contest.taskgroups.push(taskgroup);
        contest.save(&conn);

        // Have the contest review start one day in the future
        let mut in_the_future = time::get_time();
        in_the_future.sec += 60 * 60 * 24;

        // ID: 6
        let mut contest = Contest::new("directory".to_string(),
                                       "future_review.yaml".to_string(),
                                       "FutureReviewContestName".to_string(),
                                       1, // Time: 1 Minute
                                       true,
                                       None,
                                       None,
                                       Some(in_the_future),
                                       None,
                                       None,
                                       None,
                                       None,
                                       false,
                                       None,
                                       None,
                                       None,
                                       None);
        let mut taskgroup = Taskgroup::new("TaskgroupName".to_string(), None);
        let task = Task::new("taskdir1".to_string(), 3); // ID: 1
        taskgroup.tasks.push(task);
        let task = Task::new("taskdir2".to_string(), 4); // ID: 2
        taskgroup.tasks.push(task);
        contest.taskgroups.push(taskgroup);
        contest.save(&conn);

        let mut config = config::read_config_from_file(Path::new("thisfileshoudnotexist.json"));
        config.port = Some(port);
        config.cookie_signing_secret = Some("testtesttesttesttesttesttesttest".to_string());
        let message = format!("Could not start server on port {}", port);
        let mut srvr = start_server(conn, config).expect(&message);

        // Message server started
        start_tx.send(()).unwrap();

        // Wait for test to finish
        stop_rx.recv().unwrap();

        srvr.close().unwrap();
    });

    // Wait for server to start
    start_rx.recv().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Run test code
    f();

    // Message test finished
    stop_tx.send(()).unwrap();
}

fn login(port: u16, client: &reqwest::Client, username: &str, password: &str) -> reqwest::Response {
    let params = [("username", username), ("password", password)];
    client.post(&format!("http://localhost:{}/login", port)).form(&params).send().unwrap()
}

fn login_code(port: u16, client: &reqwest::Client, code: &str) -> reqwest::Response {
    let params = [("code", code)];
    client.post(&format!("http://localhost:{}/clogin", port)).form(&params).send().unwrap()
}

#[test]
fn start_server_and_check_requests() {
    start_server_and_fn(8080,
                        |_| {},
                        || {
                            let mut resp = reqwest::get("http://localhost:8080").unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert!(content.contains("Jugendwettbewerb Informatik</h1>"));
                            assert!(!content.contains("Error"));
                            assert!(!content.contains("Gruppenverwaltung"));

                            let mut resp = reqwest::get("http://localhost:8080/contest").unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert!(content.contains("<h1>Wettbewerbe</h1>"));
                            assert!(!content.contains("Error"));

                            let mut resp = reqwest::get("http://localhost:8080/group").unwrap();
                            let content = resp.text().unwrap();
                            assert!(content.contains("<h1>Login</h1>"));
                        })
}

#[test]
fn check_login_wrong_credentials() {
    start_server_and_fn(8081,
                        |_| {},
                        || {
                            let client = reqwest::Client::new();

                            let mut resp = login(8081, &client, "nonexistingusername", "wrongpassword");
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert!(content.contains("<h1>Login</h1>"));
                            assert!(content.contains("Login fehlgeschlagen."));
                            assert!(!content.contains("Error"));

                            let mut resp = login_code(8081, &client, "g23AgaV");
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert!(content.contains("<h1>Login</h1>"));
                            assert!(content.contains("Kein gültiger Code."));
                            assert!(!content.contains("Error"));

                            let mut resp = login_code(8081, &client, "u9XuAbH7p");
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert!(content.contains("<h1>Login</h1>"));
                            assert!(content.contains("Kein gültiger Code."));
                            assert!(!content.contains("Error"));
                        })
}

#[test]
fn check_login() {
    start_server_and_fn(8082,
                        |conn| {
                            addsimpleuser(conn, "testusr".to_string(), "testpw".to_string(), false, false);
                        },
                        || {
                            let client = reqwest::Client::builder().cookie_store(true)
                                                                   .redirect(reqwest::RedirectPolicy::none())
                                                                   .build()
                                                                   .unwrap();

                            let mut resp = login(8082, &client, "testusr", "testpw");
                            assert_eq!(resp.status(), StatusCode::FOUND);

                            let content = resp.text().unwrap();
                            assert!(!content.contains("Error"));

                            let mut set_cookie = resp.headers().get_all("Set-Cookie").iter();
                            assert!(set_cookie.next().is_some());
                            assert!(set_cookie.next().is_none());

                            let location = resp.headers().get(reqwest::header::LOCATION).unwrap().to_str().unwrap();
                            assert_eq!(location, "http://localhost:8082/");

                            let mut resp = client.get(location).send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert!(!content.contains("Error"));
                            assert!(!content.contains("Gruppenverwaltung"));
                            assert!(content.contains("Eingeloggt als <em>testusr</em>"));
                            assert!(content.contains("Jugendwettbewerb Informatik</h1>"));
                        })
}

#[test]
fn check_logout() {
    start_server_and_fn(8083,
                        |conn| {
                            addsimpleuser(conn, "testusr".to_string(), "testpw".to_string(), false, false);
                        },
                        || {
                            let client = reqwest::Client::builder().cookie_store(true)
                                                                   .redirect(reqwest::RedirectPolicy::none())
                                                                   .build()
                                                                   .unwrap();

                            let resp = login(8083, &client, "testusr", "testpw");
                            assert_eq!(resp.status(), StatusCode::FOUND);

                            let resp = client.get("http://localhost:8083/logout").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::FOUND);

                            let mut resp = client.get("http://localhost:8083").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert!(content.contains("Benutzername"));
                            assert!(content.contains("Passwort"));
                            assert!(content.contains("Gruppencode / Teilnahmecode"));
                            assert!(content.contains("Jugendwettbewerb Informatik</h1>"));
                        })
}

#[test]
fn check_group_creation_and_group_code_login() {
    start_server_and_fn(8084,
                        |conn| {
                            addsimpleuser(conn, "testusr".to_string(), "testpw".to_string(), true, false);
                        },
                        || {
                            let client = reqwest::Client::builder().cookie_store(true)
                                                                   .redirect(reqwest::RedirectPolicy::none())
                                                                   .build()
                                                                   .unwrap();

                            let resp = login(8084, &client, "testusr", "testpw");
                            assert_eq!(resp.status(), StatusCode::FOUND);

                            let mut resp = client.get("http://localhost:8084").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert!(content.contains("[Lehrer]"));
                            assert!(content.contains("Gruppenverwaltung"));

                            let mut resp = client.get("http://localhost:8084/group/").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert!(content.contains("Gruppe anlegen"));

                            let params =
                                [("name", "WrongGroupname"), ("tag", "WrongMarker"), ("csrf_token", "76CfTPJaoz")];
                            let resp = client.post("http://localhost:8084/group/").form(&params).send().unwrap();
                            assert_eq!(resp.status(), StatusCode::FORBIDDEN);

                            let pos = content.find("type=\"hidden\" name=\"csrf_token\" value=\"")
                                             .expect("CSRF-Token not found");
                            let csrf = &content[pos + 39..pos + 49];
                            let params = [("name", "Groupname"), ("tag", "Marker"), ("csrf_token", csrf)];
                            let resp = client.post("http://localhost:8084/group/").form(&params).send().unwrap();
                            assert_eq!(resp.status(), StatusCode::FOUND);

                            let mut resp = client.get("http://localhost:8084/group/").send().unwrap();
                            let content = resp.text().unwrap();
                            assert!(!content.contains("WrongGroupname"));

                            let pos =
                                content.find("<td><a href=\"/group/1\">Groupname</a></td>").expect("Group not found");
                            let groupcode = &content[pos + 58..pos + 65];

                            // New client to test group code login
                            let client = reqwest::Client::builder().cookie_store(true)
                                                                   .redirect(reqwest::RedirectPolicy::none())
                                                                   .build()
                                                                   .unwrap();

                            let resp = login_code(8084, &client, groupcode);
                            assert_eq!(resp.status(), StatusCode::FOUND);

                            let mut set_cookie = resp.headers().get_all("Set-Cookie").iter();
                            assert!(set_cookie.next().is_some());
                            assert!(set_cookie.next().is_none());

                            let location = resp.headers().get(reqwest::header::LOCATION).unwrap().to_str().unwrap();
                            assert_eq!(location, "http://localhost:8084/profile?status=firstlogin");

                            let mut resp = client.get(location).send().unwrap();
                            let content = resp.text().unwrap();

                            let pos = content.find("<p>Login-Code: ").expect("Logincode not found");
                            let logincode = &content[pos + 15..pos + 24];

                            let pos = content.find("type=\"hidden\" name=\"csrf_token\" value=\"")
                                             .expect("CSRF-Token not found");
                            let csrf = &content[pos + 39..pos + 49];
                            let params = [("firstname", "FirstName"),
                                          ("lastname", "LastName"),
                                          ("grade", "8"),
                                          ("sex", "2"),
                                          ("csrf_token", csrf)];
                            let resp = client.post("http://localhost:8084/profile").form(&params).send().unwrap();
                            assert_eq!(resp.status(), StatusCode::FOUND);

                            let location = resp.headers().get(reqwest::header::LOCATION).unwrap().to_str().unwrap();
                            assert_eq!(location, "http://localhost:8084/profile?status=DataChanged");

                            // New client to test login code login
                            let client = reqwest::Client::builder().cookie_store(true)
                                                                   .redirect(reqwest::RedirectPolicy::none())
                                                                   .build()
                                                                   .unwrap();

                            let resp = login_code(8084, &client, logincode);
                            assert_eq!(resp.status(), StatusCode::FOUND);

                            let location = resp.headers().get(reqwest::header::LOCATION).unwrap().to_str().unwrap();
                            assert_eq!(location, "http://localhost:8084/");

                            let mut resp = client.get(location).send().unwrap();
                            let content = resp.text().unwrap();
                            assert!(content.contains("Eingeloggt als <em></em>"));
                            println!("{}", content);
                            assert!(content.contains("FirstName LastName"));
                        })
}

#[test]
fn check_contest_start() {
    start_server_and_fn(8085,
                        |conn| {
                            addsimpleuser(conn, "testusr".to_string(), "testpw".to_string(), false, false);
                        },
                        || {
                            let client = reqwest::Client::builder().cookie_store(true)
                                                                   .redirect(reqwest::RedirectPolicy::none())
                                                                   .build()
                                                                   .unwrap();

                            let resp = login(8085, &client, "testusr", "testpw");
                            assert_eq!(resp.status(), StatusCode::FOUND);

                            let mut resp = client.get("http://localhost:8085/contest/").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert!(content.contains("PublicContestName"));
                            assert!(content.contains("InfiniteContestName"));
                            assert!(!content.contains("PrivateContestName"));
                            assert!(!content.contains("WrongContestName"));
                            assert!(!content.contains("RenamedContestName"));
                            assert!(content.contains("<a href=\"/contest/1\">PublicContestName</a>"));

                            let mut resp = client.get("http://localhost:8085/contest/1").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert!(content.contains("PublicContestName"));
                            assert!(!content.contains("InfiniteContestName"));
                            assert!(!content.contains("PrivateContestName"));
                            assert!(!content.contains("WrongContestName"));
                            assert!(!content.contains("RenamedContestName"));

                            let params = [("csrf_token", "76CfTPJaoz")];
                            let resp = client.post("http://localhost:8085/contest/1").form(&params).send().unwrap();
                            assert_eq!(resp.status(), StatusCode::FORBIDDEN);

                            let pos = content.find("type=\"hidden\" name=\"csrf_token\" value=\"")
                                             .expect("CSRF-Token not found");
                            let csrf = &content[pos + 39..pos + 49];
                            let params = [("csrf_token", csrf)];
                            let resp = client.post("http://localhost:8085/contest/1").form(&params).send().unwrap();
                            assert_eq!(resp.status(), StatusCode::FOUND);

                            let mut resp = client.get("http://localhost:8085/contest/1").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert!(content.contains("<a href=\"/task/1\">☆☆☆</a></li>"));
                            assert!(content.contains("<a href=\"/task/2\">☆☆☆☆</a></li>"));
                        })
}

#[test]
fn check_task_load_save() {
    start_server_and_fn(8086,
                        |_| {},
                        || {
                            let client = reqwest::Client::builder().cookie_store(true)
                                                                   .redirect(reqwest::RedirectPolicy::none())
                                                                   .build()
                                                                   .unwrap();

                            let resp = client.get("http://localhost:8086/contest/3").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let mut resp = client.get("http://localhost:8086/task/5").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            let pos = content.find("#taskid=5&csrftoken=").expect("CSRF-Token not found");
                            let csrf = &content[pos + 20..pos + 30];

                            let mut resp = client.get("http://localhost:8086/load/5").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert_eq!(content, "{}");

                            let params = [("data", "WrongData"), ("grade", "1"), ("csrf_token", "FNQU4QsEMY")];
                            let resp = client.post("http://localhost:8086/save/5").form(&params).send().unwrap();
                            assert_eq!(resp.status(), StatusCode::FORBIDDEN);

                            // Check that the illegitimate request did not actually change anything
                            let mut resp = client.get("http://localhost:8086/load/5").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert_eq!(content, "{}");

                            let mut resp = client.get("http://localhost:8086/contest/3").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert!(content.contains("<a href=\"/task/5\">☆☆☆</a></li>"));
                            assert!(content.contains("<a href=\"/task/6\">☆☆☆☆</a></li>"));

                            let params = [("data", "SomeData"), ("grade", "67"), ("csrf_token", csrf)];
                            let mut resp = client.post("http://localhost:8086/save/5").form(&params).send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert_eq!(content, "{}");

                            let mut resp = client.get("http://localhost:8086/load/5").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert_eq!(content, "SomeData");

                            let mut resp = client.get("http://localhost:8086/contest/3").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert!(content.contains("<a href=\"/task/5\">★★☆</a></li>"));
                            assert!(content.contains("<a href=\"/task/6\">☆☆☆☆</a></li>"));
                        })
}

#[test]
fn check_task_load_save_logged_in() {
    start_server_and_fn(8087,
                        |conn| {
                            addsimpleuser(conn, "testusr".to_string(), "testpw".to_string(), false, false);
                        },
                        || {
                            let client = reqwest::Client::builder().cookie_store(true)
                                                                   .redirect(reqwest::RedirectPolicy::none())
                                                                   .build()
                                                                   .unwrap();

                            let resp = login(8087, &client, "testusr", "testpw");
                            assert_eq!(resp.status(), StatusCode::FOUND);

                            let mut resp = client.get("http://localhost:8087/contest/1").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            let pos = content.find("type=\"hidden\" name=\"csrf_token\" value=\"")
                                             .expect("CSRF-Token not found");
                            let csrf = &content[pos + 39..pos + 49];
                            let params = [("csrf_token", csrf)];
                            let resp = client.post("http://localhost:8087/contest/1").form(&params).send().unwrap();
                            assert_eq!(resp.status(), StatusCode::FOUND);

                            let mut resp = client.get("http://localhost:8087/task/1").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            let pos = content.find("#taskid=1&csrftoken=").expect("CSRF-Token not found");
                            let csrf = &content[pos + 20..pos + 30];

                            let mut resp = client.get("http://localhost:8087/load/1").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert_eq!(content, "{}");

                            let params = [("data", "WrongData"), ("grade", "1"), ("csrf_token", "FNQU4QsEMY")];
                            let resp = client.post("http://localhost:8087/save/1").form(&params).send().unwrap();
                            assert_eq!(resp.status(), StatusCode::FORBIDDEN);

                            // Check that the illigal request did not actually change anything
                            let mut resp = client.get("http://localhost:8087/load/1").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert_eq!(content, "{}");

                            let mut resp = client.get("http://localhost:8087/contest/1").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert!(content.contains("<a href=\"/task/1\">☆☆☆</a></li>"));
                            assert!(content.contains("<a href=\"/task/2\">☆☆☆☆</a></li>"));

                            let params = [("data", "SomeData"), ("grade", "67"), ("csrf_token", csrf)];
                            let mut resp = client.post("http://localhost:8087/save/1").form(&params).send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert_eq!(content, "{}");

                            let mut resp = client.get("http://localhost:8087/load/1").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert_eq!(content, "SomeData");

                            let mut resp = client.get("http://localhost:8087/contest/1").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert!(content.contains("<a href=\"/task/1\">★★☆</a></li>"));
                            assert!(content.contains("<a href=\"/task/2\">☆☆☆☆</a></li>"));
                        })
}

#[test]
fn check_taskgroup_rename() {
    start_server_and_fn(8088,
                        |_| {},
                        || {
                            let client = reqwest::Client::builder().cookie_store(true)
                                                                   .redirect(reqwest::RedirectPolicy::none())
                                                                   .build()
                                                                   .unwrap();

                            let mut resp = client.get("http://localhost:8088/contest/3").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert!(content.contains("TaskgroupNewName"));
                            assert!(!content.contains("TaskgroupRenameName"));

                            let mut resp = client.get("http://localhost:8088/task/5").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert!(content.contains("TaskgroupNewName"));
                            assert!(!content.contains("TaskgroupRenameName"));
                        })
}

#[test]
fn check_admin_interface_link() {
    start_server_and_fn(8089,
                        |conn| {
                            addsimpleuser(conn, "testadm".to_string(), "testpw1".to_string(), false, true);
                            addsimpleuser(conn, "testusr".to_string(), "testpw2".to_string(), false, false);
                            addsimpleuser(conn, "testtch".to_string(), "testpw3".to_string(), true, false);
                        },
                        || {
                            let client = reqwest::Client::builder().cookie_store(true)
                                                                   .redirect(reqwest::RedirectPolicy::none())
                                                                   .build()
                                                                   .unwrap();

                            let resp = login(8089, &client, "testadm", "testpw1");
                            assert_eq!(resp.status(), StatusCode::FOUND);

                            let mut resp = client.get("http://localhost:8089/").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert!(content.contains("Administration"));
                            assert!(content.contains("<a href=\"/admin/\""));

                            let client = reqwest::Client::builder().cookie_store(true)
                                                                   .redirect(reqwest::RedirectPolicy::none())
                                                                   .build()
                                                                   .unwrap();

                            let mut resp = client.get("http://localhost:8089/").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert!(!content.contains("Administration"));
                            assert!(!content.contains("<a href=\"/admin/\""));

                            let client = reqwest::Client::builder().cookie_store(true)
                                                                   .redirect(reqwest::RedirectPolicy::none())
                                                                   .build()
                                                                   .unwrap();

                            let mut resp = login(8089, &client, "testusr", "testpw2");
                            assert_eq!(resp.status(), StatusCode::FOUND);

                            println!("{}", resp.text().unwrap());

                            let mut resp = client.get("http://localhost:8089/").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert!(!content.contains("Administration"));
                            assert!(!content.contains("<a href=\"/admin/\""));

                            let client = reqwest::Client::builder().cookie_store(true)
                                                                   .redirect(reqwest::RedirectPolicy::none())
                                                                   .build()
                                                                   .unwrap();

                            let mut resp = login(8089, &client, "testtch", "testpw3");
                            assert_eq!(resp.status(), StatusCode::FOUND);

                            println!("{}", resp.text().unwrap());

                            let mut resp = client.get("http://localhost:8089/").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert!(!content.contains("Administration"));
                            assert!(!content.contains("<a href=\"/admin/\""));
                        })
}

#[test]
fn check_admin_interface_access() {
    start_server_and_fn(8090,
                        |conn| {
                            addsimpleuser(conn, "testadm".to_string(), "testpw1".to_string(), false, true);
                            addsimpleuser(conn, "testusr".to_string(), "testpw2".to_string(), false, false);
                            addsimpleuser(conn, "testtch".to_string(), "testpw3".to_string(), true, false);
                        },
                        || {
                            let client = reqwest::Client::builder().cookie_store(true)
                                                                   .redirect(reqwest::RedirectPolicy::none())
                                                                   .build()
                                                                   .unwrap();

                            let resp = login(8090, &client, "testadm", "testpw1");
                            assert_eq!(resp.status(), StatusCode::FOUND);

                            let mut resp = client.get("http://localhost:8090/admin").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert!(content.contains("Administration"));
                            assert!(content.contains("Admin-Suche"));
                            assert!(content.contains("Wettbewerbs-Export"));

                            let client = reqwest::Client::builder().cookie_store(true)
                                                                   .redirect(reqwest::RedirectPolicy::none())
                                                                   .build()
                                                                   .unwrap();

                            let mut resp = client.get("http://localhost:8090/admin").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::FOUND);

                            let content = resp.text().unwrap();
                            assert!(!content.contains("Administration"));
                            assert!(!content.contains("Admin-Suche"));
                            assert!(!content.contains("Wettbewerbs-Export"));

                            let client = reqwest::Client::builder().cookie_store(true)
                                                                   .redirect(reqwest::RedirectPolicy::none())
                                                                   .build()
                                                                   .unwrap();

                            let mut resp = login(8090, &client, "testusr", "testpw2");
                            assert_eq!(resp.status(), StatusCode::FOUND);

                            println!("{}", resp.text().unwrap());

                            let mut resp = client.get("http://localhost:8090/admin").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

                            let content = resp.text().unwrap();
                            assert!(!content.contains("Administration"));
                            assert!(!content.contains("Admin-Suche"));
                            assert!(!content.contains("Wettbewerbs-Export"));

                            let client = reqwest::Client::builder().cookie_store(true)
                                                                   .redirect(reqwest::RedirectPolicy::none())
                                                                   .build()
                                                                   .unwrap();

                            let mut resp = login(8090, &client, "testtch", "testpw3");
                            assert_eq!(resp.status(), StatusCode::FOUND);

                            println!("{}", resp.text().unwrap());

                            let mut resp = client.get("http://localhost:8090/admin").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

                            let content = resp.text().unwrap();
                            assert!(!content.contains("Administration"));
                            assert!(!content.contains("Admin-Suche"));
                            assert!(!content.contains("Wettbewerbs-Export"));
                        })
}

#[test]
fn check_cleanup() {
    start_server_and_fn(8091,
                        |conn| {
                            let ago170days = Some(time::get_time() - time::Duration::days(170));
                            let ago190days = Some(time::get_time() - time::Duration::days(190));

                            let mut test_user = conn.new_session("");
                            test_user.username = Some("testusr".to_string());
                            test_user.set_password(&"testpw").expect("Set Password did not work correctly.");
                            conn.session_set_activity_dates(test_user.id, ago190days, ago190days, ago190days);
                            conn.save_session(test_user);

                            let mut test_user = conn.new_session("");
                            test_user.firstname = Some("firstname".to_string());
                            test_user.lastname = Some("teststdold".to_string());
                            test_user.logincode = Some("logincode1".to_string());
                            test_user.managed_by = Some(1); // Fake id, should this group really exist?
                            conn.session_set_activity_dates(test_user.id, ago190days, ago190days, ago190days);
                            conn.save_session(test_user);

                            let mut test_user = conn.new_session("");
                            test_user.firstname = Some("firstname".to_string());
                            test_user.lastname = Some("teststdnew".to_string());
                            test_user.logincode = Some("logincode2".to_string());
                            test_user.managed_by = Some(1);
                            conn.session_set_activity_dates(test_user.id, ago190days, ago170days, ago190days);
                            conn.save_session(test_user);

                            addsimpleuser(conn, "testadm".to_string(), "testpw1".to_string(), false, true);
                        },
                        || {
                            let client = reqwest::Client::builder().cookie_store(true)
                                                                   .redirect(reqwest::RedirectPolicy::none())
                                                                   .build()
                                                                   .unwrap();
                            // Login as Admin
                            let resp = login(8091, &client, "testadm", "testpw1");
                            assert_eq!(resp.status(), StatusCode::FOUND);

                            // Check old account still existing
                            let mut resp = client.get("http://localhost:8091/admin/user/2").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert!(content.contains("teststdold"));

                            // Delete old data
                            let mut resp = client.get("http://localhost:8091/admin/cleanup").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert!(content.contains("Alte Daten löschen"));

                            let pos = content.find("type=\"hidden\" name=\"csrf_token\" value=\"")
                                             .expect("CSRF-Token not found");
                            let csrf = &content[pos + 39..pos + 49];
                            let params = [("csrf_token", csrf)];
                            let mut resp =
                                client.post("http://localhost:8091/admin/cleanup/hard").form(&params).send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert_eq!(content, "{\"status\":\"ok\",\"n_user\":1,\"n_group\":0,\"n_teacher\":0,\"n_other\":0}\n");

                            // Check old account no longer existing
                            let mut resp = client.get("http://localhost:8091/admin/user/2").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

                            let content = resp.text().unwrap();
                            assert!(!content.contains("teststdold"));

                            // Logout as admin
                            let resp = client.get("http://localhost:8091/logout").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::FOUND);

                            // Check login with old account no longer possible
                            let resp = login_code(8091, &client, "logincode1");
                            assert_eq!(resp.status(), StatusCode::OK);

                            let mut resp = client.get("http://localhost:8091").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);
                            let content = resp.text().unwrap();
                            assert!(!content.contains("Eingeloggt als "));
                            assert!(!content.contains("teststdold"));

                            let resp = client.get("http://localhost:8091/logout").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::FOUND);

                            // Check login with new account still possible
                            let resp = login_code(8091, &client, "logincode2");
                            assert_eq!(resp.status(), StatusCode::FOUND);

                            let mut resp = client.get("http://localhost:8091").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);
                            let content = resp.text().unwrap();
                            assert!(content.contains("Eingeloggt als "));
                            assert!(content.contains("teststdnew"));

                            let resp = client.get("http://localhost:8091/logout").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::FOUND);

                            // Check login with new account still possible
                            let resp = login(8091, &client, "testusr", "testpw");
                            assert_eq!(resp.status(), StatusCode::FOUND);

                            let mut resp = client.get("http://localhost:8091").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);
                            let content = resp.text().unwrap();
                            assert!(content.contains("Eingeloggt als <em>testusr</em>"));

                            let resp = client.get("http://localhost:8091/logout").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::FOUND);
                        })
}

#[test]
fn check_contest_requirement() {
    start_server_and_fn(8092,
                        |conn| {
                            addsimpleuser(conn, "testusr".to_string(), "testpw".to_string(), false, false);
                        },
                        || {
                            let client = reqwest::Client::builder().cookie_store(true)
                                                                   .redirect(reqwest::RedirectPolicy::none())
                                                                   .build()
                                                                   .unwrap();

                            let resp = login(8092, &client, "testusr", "testpw");
                            assert_eq!(resp.status(), StatusCode::FOUND);

                            // Check contest can not be started
                            let mut resp = client.get("http://localhost:8092/contest/4").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);
                            let content = resp.text().unwrap();

                            assert!(!content.contains("csrf_token"));
                            assert!(!content.contains("<a href=\"/task/7\">☆☆☆</a></li>"));
                            assert!(!content.contains("<a href=\"/task/8\">☆☆☆☆</a></li>"));

                            // Start other contest
                            let mut resp = client.get("http://localhost:8092/contest/1").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);
                            let content = resp.text().unwrap();

                            let pos = content.find("type=\"hidden\" name=\"csrf_token\" value=\"")
                                             .expect("CSRF-Token not found");
                            let csrf = &content[pos + 39..pos + 49];
                            let params = [("csrf_token", csrf)];
                            let resp = client.post("http://localhost:8092/contest/1").form(&params).send().unwrap();
                            assert_eq!(resp.status(), StatusCode::FOUND);

                            let mut resp = client.get("http://localhost:8092/contest/1").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert!(content.contains("<a href=\"/task/1\">☆☆☆</a></li>"));
                            assert!(content.contains("<a href=\"/task/2\">☆☆☆☆</a></li>"));

                            // Check contest can be started now
                            let mut resp = client.get("http://localhost:8092/contest/4").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);
                            let content = resp.text().unwrap();

                            let pos = content.find("type=\"hidden\" name=\"csrf_token\" value=\"")
                                             .expect("CSRF-Token not found");
                            let csrf = &content[pos + 39..pos + 49];
                            let params = [("csrf_token", csrf)];
                            let resp = client.post("http://localhost:8092/contest/4").form(&params).send().unwrap();
                            assert_eq!(resp.status(), StatusCode::FOUND);

                            let mut resp = client.get("http://localhost:8092/contest/4").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert!(content.contains("<a href=\"/task/7\">☆☆☆</a></li>"));
                            assert!(content.contains("<a href=\"/task/8\">☆☆☆☆</a></li>"));
                        })
}

#[test]
fn check_group_creation_and_group_code_login_no_data() {
    start_server_and_fn(8093,
                        |conn| {
                            addsimpleuser(conn, "testusr".to_string(), "testpw".to_string(), true, false);
                        },
                        || {
                            let client = reqwest::Client::builder().cookie_store(true)
                                                                   .redirect(reqwest::RedirectPolicy::none())
                                                                   .build()
                                                                   .unwrap();

                            let resp = login(8093, &client, "testusr", "testpw");
                            assert_eq!(resp.status(), StatusCode::FOUND);

                            let mut resp = client.get("http://localhost:8093").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert!(content.contains("[Lehrer]"));
                            assert!(content.contains("Gruppenverwaltung"));

                            let mut resp = client.get("http://localhost:8093/group/").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert!(content.contains("Gruppe anlegen"));

                            let params =
                                [("name", "WrongGroupname"), ("tag", "WrongMarker"), ("csrf_token", "76CfTPJaoz")];
                            let resp = client.post("http://localhost:8093/group/").form(&params).send().unwrap();
                            assert_eq!(resp.status(), StatusCode::FORBIDDEN);

                            let pos = content.find("type=\"hidden\" name=\"csrf_token\" value=\"")
                                             .expect("CSRF-Token not found");
                            let csrf = &content[pos + 39..pos + 49];
                            let params = [("name", "Groupname"), ("tag", "Marker"), ("csrf_token", csrf)];
                            let resp = client.post("http://localhost:8093/group/").form(&params).send().unwrap();
                            assert_eq!(resp.status(), StatusCode::FOUND);

                            let mut resp = client.get("http://localhost:8093/group/").send().unwrap();
                            let content = resp.text().unwrap();
                            assert!(!content.contains("WrongGroupname"));

                            let pos =
                                content.find("<td><a href=\"/group/1\">Groupname</a></td>").expect("Group not found");
                            let groupcode = &content[pos + 58..pos + 65];

                            // New client to test group code login
                            let client = reqwest::Client::builder().cookie_store(true)
                                                                   .redirect(reqwest::RedirectPolicy::none())
                                                                   .build()
                                                                   .unwrap();

                            let resp = login_code(8093, &client, groupcode);
                            assert_eq!(resp.status(), StatusCode::FOUND);

                            let mut set_cookie = resp.headers().get_all("Set-Cookie").iter();
                            assert!(set_cookie.next().is_some());
                            assert!(set_cookie.next().is_none());

                            let location = resp.headers().get(reqwest::header::LOCATION).unwrap().to_str().unwrap();
                            assert_eq!(location, "http://localhost:8093/profile?status=firstlogin");

                            let mut resp = client.get(location).send().unwrap();
                            let content = resp.text().unwrap();

                            let pos = content.find("<p>Login-Code: ").expect("Logincode not found");
                            let logincode = &content[pos + 15..pos + 24];

                            // New client to test login code login
                            let client = reqwest::Client::builder().cookie_store(true)
                                                                   .redirect(reqwest::RedirectPolicy::none())
                                                                   .build()
                                                                   .unwrap();

                            let resp = login_code(8093, &client, logincode);
                            assert_eq!(resp.status(), StatusCode::FOUND);

                            let location = resp.headers().get(reqwest::header::LOCATION).unwrap().to_str().unwrap();
                            assert_eq!(location, "http://localhost:8093/");

                            // Client is forwarded to login page?
                            let resp = client.get("http://localhost:8093/").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::FOUND);

                            let location = resp.headers().get(reqwest::header::LOCATION).unwrap().to_str().unwrap();
                            assert_eq!(location, "http://localhost:8093/profile?status=firstlogin");
                        })
}

#[test]
fn check_contest_timelimit_student() {
    start_server_and_fn(
                        8094,
                        |conn| {
                            addsimpleuser(conn, "testusr".to_string(), "testpw".to_string(), false, false);

                            let mut more_than_one_minute_ago = time::get_time();
                            // Have the contest started more than a minute ago.
                            more_than_one_minute_ago.sec -= 90;
                            conn.execute(
                                         "INSERT INTO participation (contest, session, start_date)
                                SELECT $1, id, $2 FROM session WHERE username = 'testusr'",
                                         &[&1, &more_than_one_minute_ago],
        )
                                .unwrap();
                        },
                        || {
                            let client = reqwest::Client::builder().cookie_store(true)
                                                                   .redirect(reqwest::RedirectPolicy::none())
                                                                   .build()
                                                                   .unwrap();

                            let resp = login(8094, &client, "testusr", "testpw");
                            assert_eq!(resp.status(), StatusCode::FOUND);

                            let resp = client.get("http://localhost:8094/contest/1").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let resp = client.get("http://localhost:8094/task/1").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::FOUND);

                            let mut resp = client.get("http://localhost:8094/profile").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            let pos = content.find("type=\"hidden\" name=\"csrf_token\" value=\"")
                                             .expect("CSRF-Token not found");
                            let csrf = &content[pos + 39..pos + 49];

                            let mut resp = client.get("http://localhost:8094/load/1").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert_eq!(content, "{}");

                            let params = [("data", "SomeData"), ("grade", "67"), ("csrf_token", csrf)];
                            let resp = client.post("http://localhost:8094/save/1").form(&params).send().unwrap();
                            assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

                            let mut resp = client.get("http://localhost:8094/load/1").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert_eq!(content, "{}");

                            let mut resp = client.get("http://localhost:8094/contest/1").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert!(content.contains("<a href=\"/task/1\">☆☆☆</a></li>"));
                            assert!(content.contains("<a href=\"/task/2\">☆☆☆☆</a></li>"));
                        },
    )
}

#[test]
fn check_contest_timelimit_teacher() {
    start_server_and_fn(
                        8095,
                        |conn| {
                            addsimpleuser(conn, "testusr".to_string(), "testpw".to_string(), true, false);

                            let mut now = time::get_time();
                            now.sec -= 90; // Have the contest started more than a minute ago.
                            conn.execute(
                                         "INSERT INTO participation (contest, session, start_date)
                                SELECT $1, id, $2 FROM session WHERE username = 'testusr'",
                                         &[&1, &now],
        )
                                .unwrap();
                        },
                        || {
                            let client = reqwest::Client::builder().cookie_store(true)
                                                                   .redirect(reqwest::RedirectPolicy::none())
                                                                   .build()
                                                                   .unwrap();

                            let resp = login(8095, &client, "testusr", "testpw");
                            assert_eq!(resp.status(), StatusCode::FOUND);

                            let resp = client.get("http://localhost:8095/contest/1").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let resp = client.get("http://localhost:8095/task/1").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let mut resp = client.get("http://localhost:8095/profile").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            let pos = content.find("type=\"hidden\" name=\"csrf_token\" value=\"")
                                             .expect("CSRF-Token not found");
                            let csrf = &content[pos + 39..pos + 49];

                            let mut resp = client.get("http://localhost:8095/load/1").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert_eq!(content, "{}");

                            let params = [("data", "SomeData"), ("grade", "67"), ("csrf_token", csrf)];
                            let mut resp = client.post("http://localhost:8095/save/1").form(&params).send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert_eq!(content, "{}");

                            let mut resp = client.get("http://localhost:8095/load/1").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert_eq!(content, "SomeData");

                            let mut resp = client.get("http://localhost:8095/contest/1").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert!(content.contains("<a href=\"/task/1\">★★☆</a></li>"));
                            assert!(content.contains("<a href=\"/task/2\">☆☆☆☆</a></li>"));
                        },
    )
}

#[test]
fn check_review_student() {
    start_server_and_fn(
                        8096,
                        |conn| {
                            addsimpleuser(conn, "testusr".to_string(), "testpw".to_string(), false, false);

                            let mut more_than_one_minute_ago = time::get_time();
                            // Have the contest started more than a minute ago.
                            more_than_one_minute_ago.sec -= 90;
                            conn.execute(
                                         "INSERT INTO participation (contest, session, start_date)
                                SELECT $1, id, $2 FROM session WHERE username = 'testusr'",
                                         &[&5, &more_than_one_minute_ago],
        )
                                .unwrap();
                        },
                        || {
                            let client = reqwest::Client::builder().cookie_store(true)
                                                                   .redirect(reqwest::RedirectPolicy::none())
                                                                   .build()
                                                                   .unwrap();

                            let resp = login(8096, &client, "testusr", "testpw");
                            assert_eq!(resp.status(), StatusCode::FOUND);

                            let mut resp = client.get("http://localhost:8096/contest/5").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert!(content.contains("Review-Modus: Du kannst die Aufgaben öffnen und bearbeiten."));

                            let mut resp = client.get("http://localhost:8096/task/9").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert!(content.contains("<em>Review-Modus</em>"));

                            let mut resp = client.get("http://localhost:8096/profile").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            let pos = content.find("type=\"hidden\" name=\"csrf_token\" value=\"")
                                             .expect("CSRF-Token not found");
                            let csrf = &content[pos + 39..pos + 49];

                            let mut resp = client.get("http://localhost:8096/load/9").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert_eq!(content, "{}");

                            let params = [("data", "SomeData"), ("grade", "67"), ("csrf_token", csrf)];
                            let resp = client.post("http://localhost:8096/save/9").form(&params).send().unwrap();
                            assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

                            let mut resp = client.get("http://localhost:8096/load/9").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert_eq!(content, "{}");

                            let mut resp = client.get("http://localhost:8096/contest/5").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert!(content.contains("<a href=\"/task/9\">☆☆☆</a></li>"));
                            assert!(content.contains("<a href=\"/task/10\">☆☆☆☆</a></li>"));
                        },
    )
}

#[test]
fn check_future_review_student() {
    start_server_and_fn(
                        8097,
                        |conn| {
                            addsimpleuser(conn, "testusr".to_string(), "testpw".to_string(), false, false);

                            let mut more_than_one_minute_ago = time::get_time();
                            // Have the contest started more than a minute ago.
                            more_than_one_minute_ago.sec -= 90;
                            conn.execute(
                                         "INSERT INTO participation (contest, session, start_date)
                                SELECT $1, id, $2 FROM session WHERE username = 'testusr'",
                                         &[&6, &more_than_one_minute_ago],
        )
                                .unwrap();
                        },
                        || {
                            let client = reqwest::Client::builder().cookie_store(true)
                                                                   .redirect(reqwest::RedirectPolicy::none())
                                                                   .build()
                                                                   .unwrap();

                            let resp = login(8097, &client, "testusr", "testpw");
                            assert_eq!(resp.status(), StatusCode::FOUND);

                            let mut resp = client.get("http://localhost:8097/contest/6").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert!(content.contains("Der Review-Modus beginnt in"));

                            let resp = client.get("http://localhost:8097/task/11").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::FOUND);

                            let mut resp = client.get("http://localhost:8097/profile").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            let pos = content.find("type=\"hidden\" name=\"csrf_token\" value=\"")
                                             .expect("CSRF-Token not found");
                            let csrf = &content[pos + 39..pos + 49];

                            let mut resp = client.get("http://localhost:8097/load/11").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert_eq!(content, "{}");

                            let params = [("data", "SomeData"), ("grade", "67"), ("csrf_token", csrf)];
                            let resp = client.post("http://localhost:8097/save/11").form(&params).send().unwrap();
                            assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

                            let mut resp = client.get("http://localhost:8097/load/11").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert_eq!(content, "{}");

                            let mut resp = client.get("http://localhost:8097/contest/6").send().unwrap();
                            assert_eq!(resp.status(), StatusCode::OK);

                            let content = resp.text().unwrap();
                            assert!(content.contains("<a href=\"/task/11\">☆☆☆</a></li>"));
                            assert!(content.contains("<a href=\"/task/12\">☆☆☆☆</a></li>"));
                        },
    )
}
