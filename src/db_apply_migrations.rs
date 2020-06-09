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

use std::fs;
use std::io::Read;

use db_conn::MedalConnection;

pub fn test<C: MedalConnection>(conn: &mut C) {
    let mut paths: Vec<_> =
        fs::read_dir(format!("migrations/{}", conn.dbtype())).unwrap()
                                                             .map(|r| r.unwrap())
                                                             .filter(|r| {
                                                                 r.path().display().to_string().ends_with(".sql")
                                                             })
                                                             .collect();
    paths.sort_by_key(|dir| dir.path());

    for path in paths {
        let filename = path.file_name().into_string().unwrap();
        if !conn.migration_already_applied(&filename) {
            let mut file = fs::File::open(path.path()).unwrap();
            let mut contents = String::new();
            file.read_to_string(&mut contents).unwrap();
            conn.apply_migration(&filename, &contents);
            println!("Found: {}. Applying …", path.path().display());
        }
        /*else { // TODO: Show in high debug level only
            println!("Found: {}. Already applied", path.path().display());
        }*/
    }
}
