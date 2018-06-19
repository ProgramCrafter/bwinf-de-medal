use db_objects::*;


pub trait MedalConnection {
    fn create() -> Self;
    fn dbtype(&self) -> &'static str;

    fn migration_already_applied(&mut self, name: &str) -> bool;   
    fn apply_migration(&mut self, name: &str, contents: String);

    fn get_session(&mut self, key: String) -> Option<SessionUser>;
    fn new_session(&mut self) -> SessionUser;
    fn get_session_or_new(&mut self, key: String) -> SessionUser;

    fn login(&mut self, session: &SessionUser, username: String, password: String) -> Result<SessionUser,()>;
    fn login_with_code(&mut self, session: &SessionUser, logincode: String) -> Result<SessionUser,()>;
    fn logout(&mut self, session: &SessionUser);

    fn load_submission(&mut self, session: &SessionUser, task: String, subtask: Option<String>) -> Submission;
    fn submit_submission(&mut self, session: &SessionUser, task: String, subtask: Option<String>, submission: Submission);

    fn get_contest_by_id(&mut self, contest_id : u32) -> Contest;
    fn get_contest_by_id_complete(&mut self, contest_id : u32) -> Contest;
    fn get_task_by_id(&mut self, task_id : u32) -> Task;
    fn get_task_by_id_complete(&mut self, task_id : u32) -> (Task, Taskgroup, Contest);

    fn get_submission_to_validate(&mut self, tasklocation: String, subtask: Option<String>) -> u32;
    fn find_next_submission_to_validate(&mut self, userid: u32, taskgroupid: u32);
}


pub trait MedalObject<T: MedalConnection> {
    fn save(&mut self, conn: &mut T);
}
