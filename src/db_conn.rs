use db_objects::*;
use functions;
use std::path::Path;

pub trait MedalConnection {
    fn create(file: &Path) -> Self;
    fn dbtype(&self) -> &'static str;

    fn migration_already_applied(&self, name: &str) -> bool;
    fn apply_migration(&mut self, name: &str, contents: &str);

    fn get_session(&self, key: &str) -> Option<SessionUser>;
    fn new_session(&self, key: &str) -> SessionUser;
    fn save_session(&self, session: SessionUser);
    fn get_session_or_new(&self, key: &str) -> SessionUser;

    fn get_user_by_id(&self, user_id: u32) -> Option<SessionUser>;
    fn get_user_and_group_by_id(&self, user_id: u32) -> Option<(SessionUser, Option<Group>)>;

    //fn login(&self, session: &SessionUser, username: String, password: String) -> Result<String,()>;

    fn login(&self, session: Option<&str>, username: &str, password: &str) -> Result<String, ()>;
    fn login_with_code(&self, session: Option<&str>, logincode: &str) -> Result<String, ()>;
    fn login_foreign(&self, session: Option<&str>, foreign_id: u32, foreign_type: functions::UserType,
                     firstname: &str, lastname: &str)
                     -> Result<String, ()>;
    fn create_user_with_groupcode(&self, session: Option<&str>, groupcode: &str) -> Result<String, ()>;
    fn logout(&self, session: &str);

    fn load_submission(&self, session: &SessionUser, task: u32, subtask: Option<&str>) -> Option<Submission>;
    fn submit_submission(&self, submission: Submission);
    fn get_grade_by_submission(&self, submission_id: u32) -> Grade;
    fn get_contest_groups_grades(&self, session_id: u32, contest_id: u32)
                                 -> (Vec<String>, Vec<(Group, Vec<(UserInfo, Vec<Grade>)>)>);
    fn get_taskgroup_user_grade(&self, session: &str, taskgroup_id: u32) -> Grade;
    fn get_contest_user_grades(&self, session: &str, contest_id: u32) -> Vec<Grade>;

    fn get_contest_list(&self) -> Vec<Contest>;
    fn get_contest_by_id(&self, contest_id: u32) -> Contest;
    fn get_contest_by_id_partial(&self, contest_id: u32) -> Contest;
    fn get_contest_by_id_complete(&self, contest_id: u32) -> Contest;
    fn get_participation(&self, session: &str, contest_id: u32) -> Option<Participation>;
    fn new_participation(&self, session: &str, contest_id: u32) -> Result<Participation, ()>;
    fn get_task_by_id(&self, task_id: u32) -> Task;
    fn get_task_by_id_complete(&self, task_id: u32) -> (Task, Taskgroup, Contest);

    fn get_submission_to_validate(&self, tasklocation: &str, subtask: Option<&str>) -> u32;
    fn find_next_submission_to_validate(&self, userid: u32, taskgroupid: u32);

    fn add_group(&self, group: &mut Group);
    fn get_groups(&self, session_id: u32) -> Vec<Group>;
    fn get_groups_complete(&self, session_id: u32) -> Vec<Group>;
    fn get_group_complete(&self, group_id: u32) -> Option<Group>;
}

pub trait MedalObject<T: MedalConnection> {
    fn save(&mut self, conn: &T);
}
