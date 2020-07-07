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

extern crate bcrypt;

use rand::{distributions::Alphanumeric, thread_rng, Rng};

use core::MedalError;
use db_objects::SessionUser;

pub fn make_ambiguous_code(len: usize) -> String {
    thread_rng().sample_iter(&Alphanumeric).take(len).collect()
}

pub fn make_unambiguous_code(len: usize) -> String {
    thread_rng().sample_iter(&Alphanumeric)
        .filter(|x| {
            let x = *x;
            !(x == 'l' || x == 'I' || x == '1' || x == 'O' || x == 'o' || x == '0')
        })
        .take(len)
        .collect()
}

pub fn make_unambiguous_code_prefix(len: usize, prefix: &str) -> String {
    let mut code = prefix.to_owned();
    code.push_str(&make_unambiguous_code(len));
    code
}

pub fn make_session_token() -> String { make_ambiguous_code(10) }

pub fn make_csrf_token() -> String { make_ambiguous_code(10) }

pub fn make_salt() -> String { make_ambiguous_code(10) }

pub fn make_filename_secret() -> String { make_ambiguous_code(10) }

pub fn make_group_code() -> String {
    make_unambiguous_code_prefix(6, "g")
}

pub fn make_login_code() -> String {
    make_unambiguous_code_prefix(8, "u")
}

pub fn hash_password(password: &str, salt: &str) -> Result<String, MedalError> {
    let password_and_salt = [password, salt].concat();
    match bcrypt::hash(password_and_salt, 5) {
        Ok(result) => Ok(result),
        Err(_) => Err(MedalError::PasswordHashingError),
    }
}

pub fn verify_password(password: &str, salt: &str, password_hash: &str) -> bool {
    let password_and_salt = [password, salt].concat();
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
