

//#[cfg(feature = "preserve_order")]
//pub type Map<K, V> = linked_hash_map::LinkedHashMap<K, V>;

extern crate linked-hash-map;

use linked_hash_map::LinkedHashMap

#[derive(Serialize, Deserialize)]
struct TaskgroupJson {
    location: String,
    stars: Option<u8>,
}

#[derive(Serialize, Deserialize)]
struct TaskgroupJson {
    name: String,
    location: Option<String>,
    variants: Option<Vec<Task>>,
    locations: Option<Vec<String>>,
}


#[derive(Serialize, Deserialize)]
struct ContestJson {
    name: String,
    participation_start: Option<DateTime<Utc>>,
    participation_end: Option<DateTime<Utc>>,
    duration_minutes: u32,
    public_listing: Option<bool>,
    tasks: LinkedHashMap<String, serde_json::Value>,
}
