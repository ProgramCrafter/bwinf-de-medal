/*  medal                                                                                                            *\
 *  Copyright (C) 2022  Bundesweite Informatikwettbewerbe, Robert Czechowski                                                            *
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

use rand::{
    distributions::{Alphanumeric, Distribution},
    thread_rng, Rng,
};

struct LowercaseAlphanumeric;
impl Distribution<char> for LowercaseAlphanumeric {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> char {
        const RANGE: u32 = 26 + 10;
        const GEN_ASCII_LOWERCASE_STR_CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";

        let var: u32 = rng.gen_range(0, RANGE);
        GEN_ASCII_LOWERCASE_STR_CHARSET[var as usize] as char
    }
}

use core::MedalError;
use db_objects::SessionUser;

pub fn make_ambiguous_code(len: usize) -> String { thread_rng().sample_iter(&Alphanumeric).take(len).collect() }

pub fn make_unambiguous_lowercase_code(len: usize) -> String {
    thread_rng().sample_iter(&LowercaseAlphanumeric)
                .filter(|x| {
                    let x = *x;
                    !(x == 'l' || x == '1' || x == 'o' || x == '0')
                })
                .take(len)
                .collect()
}

pub fn make_unambiguous_lowercase_code_prefix(len: usize, prefix: &str) -> String {
    let mut code = prefix.to_owned();
    code.push_str(&make_unambiguous_lowercase_code(len));
    code
}

pub fn make_session_token() -> String { make_ambiguous_code(10) }

pub fn make_csrf_token() -> String { make_ambiguous_code(10) }

pub fn make_salt() -> String { make_ambiguous_code(10) }

pub fn make_filename_secret() -> String { make_ambiguous_code(10) }

pub fn make_groupcode() -> String { make_unambiguous_lowercase_code_prefix(7, "g") } // 1 week @ 10/s, about 5700 groups

pub fn make_logincode() -> String { make_unambiguous_lowercase_code_prefix(9, "u") } // 1 y @ 10/s, about 110000 users

pub fn make_admincode() -> String { make_unambiguous_lowercase_code_prefix(10, "a") } // 3.6 My @ 10/s, 1 admin

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
    fn set_password(&mut self, password: &str) -> Option<()>;
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
