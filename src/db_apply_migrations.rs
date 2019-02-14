use std::fs;
use std::io::Read;

use db_conn::MedalConnection;


pub fn test<C: MedalConnection>(conn: &mut C) {
    let mut paths: Vec<_> =
        fs::read_dir(format!("migrations/{}", conn.dbtype()))
        .unwrap()
        .map(|r| r.unwrap())
        .filter(|r| r.path()
                .display()
                .to_string()
                .ends_with(".sql"))
        .collect();
    paths.sort_by_key(|dir| dir.path());

    for path in paths {        
        let filename = path.file_name().into_string().unwrap();
        if ! conn.migration_already_applied(&filename) {
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
