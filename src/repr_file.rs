use std::collections::HashMap;

pub trait ReprFile: Sized {
    fn save(&self) -> String;
    fn load(src: &str) -> Result<Self, String>;
}

impl ReprFile for Vec<String> {
    fn save(&self) -> String {
        let mut o = String::new();
        for line in self {
            o.push_str(line);
        }
        o
    }
    fn load(src: &str) -> Result<Self, String> {
        Ok(src.lines().map(|v| v.to_owned()).collect())
    }
}
impl ReprFile for HashMap<String, String> {
    fn save(&self) -> String {
        let mut o = String::new();
        for (key, value) in self {
            o.push_str(key);
            o.push('=');
            o.push_str(value);
            o.push('\n');
        }
        o
    }
    fn load(src: &str) -> Result<Self, String> {
        let mut o = HashMap::new();
        for line in src.lines() {
            if !line.is_empty() {
                if let Some((key, value)) = line.split_once('=') {
                    o.insert(key.to_owned(), value.to_owned());
                } else {
                    return Err(format!(
                        "Nonempty line didn't contain the required = char! (line: {line:?})"
                    ));
                }
            }
        }
        Ok(o)
    }
}
