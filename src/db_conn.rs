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

use config;
use db_objects::*;

#[derive(Debug)]
pub enum SignupResult {
    SignedUp,
    EmailTaken,
    UsernameTaken,
    UserLoggedIn,
    EmptyFields,
}

/// This trait abstracts the database connection and provides function for all actions to be performed on the database
/// in the medal platform.
pub trait MedalConnection {
    fn reconnect(config: &config::Config) -> Self;

    fn dbtype(&self) -> &'static str;

    fn migration_already_applied(&self, name: &str) -> bool;
    fn apply_migration(&mut self, name: &str, contents: &str);

    /// Try to get session associated to the session token `key`.
    ///
    /// Returns an `Option` that can contain the `SessionUser` of the session if the session exists and is not expired or
    /// `None` otherwise.
    fn get_session(&self, key: &str) -> Option<SessionUser>;

    /// Create a new anonymous session with the session token `key`.
    ///
    /// Returns the `SessionUser` of the session.
    fn new_session(&self, key: &str) -> SessionUser;
    /// Set activity date (for testing purposes)
    fn session_set_activity_dates(&self, session_id: i32, account_created: Option<time::Timespec>,
                                  last_login: Option<time::Timespec>, last_activity: Option<time::Timespec>);
    /// Saves the session data of `session` in the database.
    fn save_session(&self, session: SessionUser);
    /// Combination of [`get_session`](#tymethod.get_session) and  [`new_session`](#tymethod.new_session).
    ///
    /// This method can still fail in case of database error in order to bubble them up to the webframework
    fn get_session_or_new(&self, key: &str) -> Result<SessionUser, ()>;

    /// Try to get session associated to the id `user_id`.
    ///
    /// Returns an `Option` that can contain the `SessionUser` of the session if the session exists or `None` otherwise.
    fn get_user_by_id(&self, user_id: i32) -> Option<SessionUser>;

    /// Try to get session and user group associated to the id `user_id`.
    ///
    /// Returns an `Option` that can contain a pair of `SessionUser` and `Option<Group>` of the session and optionally
    /// the group if the session exists or `None` otherwise.
    fn get_user_and_group_by_id(&self, user_id: i32) -> Option<(SessionUser, Option<Group>)>;

    /// Try to login in the user with `username` and `password`.
    ///
    /// Returns a `Result` that either contains the new session token for the user if the login was successfull or no
    /// value if the login was not successfull.
    fn login(&self, session: Option<&str>, username: &str, password: &str) -> Result<String, ()>;
    fn login_with_code(&self, session: Option<&str>, logincode: &str) -> Result<String, ()>;
    fn login_foreign(&self, session: Option<&str>, provider_id: &str, foreign_id: &str,
                     _: (bool, bool, &str, &str, Option<i32>))
                     -> Result<(String, Option<time::Timespec>), ()>;
    fn create_user_with_groupcode(&self, session: Option<&str>, groupcode: &str) -> Result<String, ()>;
    fn create_group_with_users(&self, group: Group);

    /// Logs out the user identified by session token `session` by resetting the uesr's session token in the database
    /// to `NULL`.
    fn logout(&self, session: &str);

    fn signup(&self, session_token: &str, username: &str, email: &str, password_hash: String, salt: &str)
              -> SignupResult;

    fn load_submission(&self, session: &SessionUser, task: i32, subtask: Option<&str>) -> Option<Submission>;
    fn get_all_submissions(&self, session_id: i32, task: i32, subtask: Option<&str>) -> Vec<Submission>;
    fn submit_submission(&self, submission: Submission);
    fn get_grade_by_submission(&self, submission_id: i32) -> Grade;
    fn get_contest_groups_grades(&self, session_id: i32, contest_id: i32)
                                 -> (Vec<String>, Vec<(Group, Vec<(UserInfo, Vec<Grade>)>)>);
    fn get_taskgroup_user_grade(&self, session: &str, taskgroup_id: i32) -> Grade;
    fn get_contest_user_grades(&self, session: &str, contest_id: i32) -> Vec<Grade>;
    fn export_contest_results_to_file(&self, contest_id: i32, taskgroups_ids: &[(i32, String)], filename: &str);

    /// Returns a `Vec` of /all/ contests ever defined.
    fn get_contest_list(&self) -> Vec<Contest>;

    /// Returns the contest identified by `contest_id` without any associated taskgroups. Panics if the contest does not
    /// exist.
    fn get_contest_by_id(&self, contest_id: i32) -> Contest;

    /// Returns the contest identified by `contest_id` with associated taskgroups but without any associated tasks of
    /// the taskgroups. Panics if the contest does not exist.
    fn get_contest_by_id_partial(&self, contest_id: i32) -> Contest;

    /// Returns the contest identified by `contest_id` with associated taskgroups and all associated tasks of the
    /// taskgroups. Panics if the contest does not exist.
    fn get_contest_by_id_complete(&self, contest_id: i32) -> Contest;

    /// Try to get the participation associated to the session id `session_id` and the contest id `contest_id`.
    ///
    /// Returns an `Option` that can contain the `Participation` if it exists or `None` otherwise.
    fn get_participation(&self, session_id: i32, contest_id: i32) -> Option<Participation>;

    /// Try to get the participation associated to the session token `session` and the contest id `contest_id`.
    ///
    /// Returns an `Option` that can contain the `Participation` if it exists or `None` otherwise.
    fn get_own_participation(&self, session: &str, contest_id: i32) -> Option<Participation>;

    /// Collect all the participation associated to the session token `session`.
    ///
    /// Returns an `Vec` that contains pairs of all participations with their associated contests.
    fn get_all_participations_complete(&self, session_id: i32) -> Vec<(Participation, Contest)>;

    fn has_participation_by_contest_file(&self, session_id: i32, location: &str, filename: &str) -> bool;

    /// Start a new participation of the session identified by the session token `session` for the contest with the
    /// contest id `contest_id`. It checks whether the session is allowed to start the participation.
    ///
    /// Returns an `Result` that either contains the new `Participation` if the checks succeded or no value if the
    /// checks failed.
    fn new_participation(&self, session: &str, contest_id: i32) -> Result<Participation, ()>;
    fn get_task_by_id(&self, task_id: i32) -> Task;
    fn get_task_by_id_complete(&self, task_id: i32) -> (Task, Taskgroup, Contest);

    fn get_submission_to_validate(&self, tasklocation: &str, subtask: Option<&str>) -> i32;
    fn find_next_submission_to_validate(&self, userid: i32, taskgroupid: i32);

    fn add_group(&self, group: &mut Group);
    fn get_groups(&self, session_id: i32) -> Vec<Group>;
    fn get_groups_complete(&self, session_id: i32) -> Vec<Group>;
    fn get_group_complete(&self, group_id: i32) -> Option<Group>;

    fn delete_user(&self, user_id: i32);
    fn delete_group(&self, group_id: i32);
    fn delete_participation(&self, user_id: i32, contest_id: i32);
    fn remove_old_users_and_groups(&self, maxstudentage: time::Timespec, maxteacherage: Option<time::Timespec>,
                                   maxage: Option<time::Timespec>)
                                   -> Result<(i32, i32, i32, i32), ()>;
    fn remove_temporary_sessions(&self, maxage: time::Timespec) -> Result<(i32,), ()>;
    fn remove_unreferenced_participation_data(&self) -> Result<(i32, i32, i32), ()>;

    fn get_search_users(
        &self, _: (Option<i32>, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>))
        -> Result<Vec<(i32, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>)>,
                  Vec<(i32, String, String, String)>>;

    fn get_debug_information(&self) -> String;

    fn reset_all_contest_visibilities(&self);
    fn reset_all_taskgroup_visibilities(&self);
}

pub trait MedalObject<T: MedalConnection> {
    fn save(&mut self, conn: &T);
}
