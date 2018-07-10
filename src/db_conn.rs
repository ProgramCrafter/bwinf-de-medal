use db_objects::*;


pub trait MedalConnection {
    fn create() -> Self;
    fn dbtype(&self) -> &'static str;

    fn migration_already_applied(&self, name: &str) -> bool;   
    fn apply_migration(&mut self, name: &str, contents: String);

    fn get_session(&self, key: String) -> Option<SessionUser>;
    fn new_session(&self) -> SessionUser;
    fn save_session(&self, session: SessionUser);
    fn get_session_or_new(&self, key: String) -> SessionUser;

    //fn login(&self, session: &SessionUser, username: String, password: String) -> Result<String,()>;

    fn login(&self, session: Option<String>, username: String, password: String) -> Result<String,()>;
    fn login_with_code(&self, session: Option<String>, logincode: String) -> Result<String,()>;
    fn create_user_with_groupcode(&self, session: Option<String>, groupcode: String) -> Result<String,()>;
    fn logout(&self, session: String);

    fn load_submission(&self, session: &SessionUser, task: u32, subtask: Option<String>) -> Option<Submission>;
    fn submit_submission(&self, submission: Submission);

    fn get_contest_list(&self) -> Vec<Contest>;
    fn get_contest_by_id(&self, contest_id: u32) -> Contest;
    fn get_contest_by_id_complete(&self, contest_id :u32) -> Contest;
    fn get_participation(&self, session: String, contest_id: u32) -> Option<Participation>;
    fn new_participation(&self, session: String, contest_id: u32) -> Result<Participation, ()>;
    fn get_task_by_id(&self, task_id: u32) -> Task;
    fn get_task_by_id_complete(&self, task_id: u32) -> (Task, Taskgroup, Contest);

    fn get_submission_to_validate(&self, tasklocation: String, subtask: Option<String>) -> u32;
    fn find_next_submission_to_validate(&self, userid: u32, taskgroupid: u32);

    fn add_group(&self, group: &mut Group);
    fn get_groups(&self, session_id: u32) -> Vec<Group>;
    fn get_groups_complete(&self, session_id: u32) -> Vec<Group>;
    fn get_group_complete(&self, group_id: u32) -> Option<Group>;    
}


pub trait MedalObject<T: MedalConnection> {
    fn save(&mut self, conn: &T);
}
