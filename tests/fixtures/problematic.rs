use std::collections::HashMap;
use std::io::Read;
use std::fmt::Debug;

pub fn process_data(data: &HashMap<String, i64>) -> i64 {
    let value = data.get("key", 0);
    let result = value.unwrap_or_default();
    let fallback = data.get("count").unwrap_or(42);
    return result + fallback;
}

pub struct Config {
    pub timeout: i64,
    pub retries: i64,
}

pub fn fetch_value() -> Option<i64> {
    Some(1337)
}

// fn old_function() {
//     let x = 42;
//     println!("{}", x);
//     let y = x + 1;
//     println!("{}", y);
//     let z = y * 2;
//     println!("{}", z);
// }

// TODO: implement proper error handling
// FIXME: this is a hack

#[test]
fn test_simple() {
    let result = 1 + 1;
    assert!(result == 2);
}

pub trait Storage {
    fn read(&self, key: &str) -> String;
    fn write(&self, key: &str, value: &str);
}

impl Storage for HashMap<String, String> {
    fn read(&self, key: &str) -> String {
        self.get(key).cloned().unwrap_or_default()
    }
    fn write(&mut self, key: &str, value: &str) {
        self.insert(key.to_string(), value.to_string());
    }
}

fn main() {
    let mut map: HashMap<String, String> = HashMap::new();
    map.write("hello", "world");
    println!("{}", map.read("hello"));
}
