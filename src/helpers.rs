extern crate bcrypt;

use rand::{distributions::Alphanumeric, thread_rng, Rng};

use core::MedalError;
use db_objects::SessionUser;

pub fn make_session_token() -> String { thread_rng().sample_iter(&Alphanumeric).take(10).collect() }

pub fn make_csrf_token() -> String { thread_rng().sample_iter(&Alphanumeric).take(10).collect() }

pub fn make_salt() -> String { thread_rng().sample_iter(&Alphanumeric).take(10).collect() }

pub fn make_group_code() -> String {
    Some('g').into_iter()
             .chain(thread_rng().sample_iter(&Alphanumeric))
             .filter(|x| {
                 let x = *x;
                 !(x == 'l' || x == 'I' || x == '1' || x == 'O' || x == 'o' || x == '0')
             })
             .take(7)
             .collect()
}

pub fn make_login_code() -> String {
    Some('u').into_iter()
             .chain(thread_rng().sample_iter(&Alphanumeric))
             .filter(|x| {
                 let x = *x;
                 !(x == 'l' || x == 'I' || x == '1' || x == 'O' || x == 'o' || x == '0')
             })
             .take(9)
             .collect()
}

pub fn hash_password(password: &str, salt: &str) -> Result<String, MedalError> {
    let password_and_salt = [password, salt].concat().to_string();
    match bcrypt::hash(password_and_salt, 5) {
        Ok(result) => Ok(result),
        Err(_) => Err(MedalError::PasswordHashingError),
    }
}

pub fn verify_password(password: &str, salt: &str, password_hash: &str) -> bool {
    let password_and_salt = [password, salt].concat().to_string();
    match bcrypt::verify(password_and_salt, password_hash) {
        Ok(result) => result,
        _ => false,
    }
}

pub trait SetPassword {
    fn set_password(&mut self, &str) -> Option<()>;
}

impl SetPassword for SessionUser {
    fn set_password(&mut self, password: &str) -> Option<()> {
        let salt: String = thread_rng().sample_iter(&Alphanumeric).take(10).collect();
        let hash = hash_password(password, &salt).ok()?;

        self.password = Some(hash);
        self.salt = Some(salt);
        Some(())
    }
}
