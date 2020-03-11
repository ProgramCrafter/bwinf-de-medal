use db_objects::*;

pub trait MedalConnection {
    fn dbtype(&self) -> &'static str;

    fn migration_already_applied(&self, name: &str) -> bool;
    fn apply_migration(&mut self, name: &str, contents: &str);

    fn get_session(&self, key: &str) -> Option<SessionUser>;
    fn new_session(&self, key: &str) -> SessionUser;
    fn save_session(&self, session: SessionUser);
    fn get_session_or_new(&self, key: &str) -> SessionUser;

    fn get_user_by_id(&self, user_id: i32) -> Option<SessionUser>;
    fn get_user_and_group_by_id(&self, user_id: i32) -> Option<(SessionUser, Option<Group>)>;

    //fn login(&self, session: &SessionUser, username: String, password: String) -> Result<String,()>;

    fn login(&self, session: Option<&str>, username: &str, password: &str) -> Result<String, ()>;
    fn login_with_code(&self, session: Option<&str>, logincode: &str) -> Result<String, ()>;
    fn login_foreign(&self, session: Option<&str>, provider_id: &str, foreign_id: &str, is_teacher: bool,
                     firstname: &str, lastname: &str)
                     -> Result<String, ()>;
    fn create_user_with_groupcode(&self, session: Option<&str>, groupcode: &str) -> Result<String, ()>;
    fn create_group_with_users(&self, group: Group);
    fn logout(&self, session: &str);

    fn load_submission(&self, session: &SessionUser, task: i32, subtask: Option<&str>) -> Option<Submission>;
    fn submit_submission(&self, submission: Submission);
    fn get_grade_by_submission(&self, submission_id: i32) -> Grade;
    fn get_contest_groups_grades(&self, session_id: i32, contest_id: i32)
                                 -> (Vec<String>, Vec<(Group, Vec<(UserInfo, Vec<Grade>)>)>);
    fn get_taskgroup_user_grade(&self, session: &str, taskgroup_id: i32) -> Grade;
    fn get_contest_user_grades(&self, session: &str, contest_id: i32) -> Vec<Grade>;

    fn get_contest_list(&self) -> Vec<Contest>;
    fn get_contest_by_id(&self, contest_id: i32) -> Contest;
    fn get_contest_by_id_partial(&self, contest_id: i32) -> Contest;
    fn get_contest_by_id_complete(&self, contest_id: i32) -> Contest;
    fn get_participation(&self, session: &str, contest_id: i32) -> Option<Participation>;
    fn new_participation(&self, session: &str, contest_id: i32) -> Result<Participation, ()>;
    fn get_task_by_id(&self, task_id: i32) -> Task;
    fn get_task_by_id_complete(&self, task_id: i32) -> (Task, Taskgroup, Contest);

    fn get_submission_to_validate(&self, tasklocation: &str, subtask: Option<&str>) -> i32;
    fn find_next_submission_to_validate(&self, userid: i32, taskgroupid: i32);

    fn add_group(&self, group: &mut Group);
    fn get_groups(&self, session_id: i32) -> Vec<Group>;
    fn get_groups_complete(&self, session_id: i32) -> Vec<Group>;
    fn get_group_complete(&self, group_id: i32) -> Option<Group>;

    fn get_debug_information(&self) -> String;

    fn reset_all_contest_visibilities(&self);
    fn reset_all_taskgroup_visibilities(&self);
}

pub trait MedalObject<T: MedalConnection> {
    fn save(&mut self, conn: &T);
}
