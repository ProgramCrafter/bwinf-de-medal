use rand::{thread_rng, Rng};

fn make_session_key() -> String {
    thread_rng().gen_ascii_chars().take(10).collect()
}
