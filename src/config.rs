use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
};

pub(crate) struct Config {
    sections: HashMap<String, HashMap<String, String>>,
}
impl Config {
    pub(crate) fn get(&self, query: &str) -> Option<&str> {
        let mut queries = query.split('.');
        match queries.next() {
            None => None,
            Some(query) => match queries.next() {
                None => None,
                Some(query2) => self
                    .sections
                    .get(query)
                    .and_then(|x| x.get(query2))
                    .map(|x| x.as_str()),
            },
        }
    }
}

pub(crate) fn parse_config_from_file(f: File) -> Config {
    let mut config = Config {
        sections: HashMap::new(),
    };
    let reader = BufReader::new(f);
    let mut current_section = String::new();
    for line in reader.lines() {
        let Ok(line) = line else {
            break;
        };
        if line.starts_with('[') && line.ends_with(']') {
            current_section = line[1..line.len() - 1].to_string();
            config
                .sections
                .insert(current_section.clone(), HashMap::new());
        } else if let Some((key, value)) = line.split_once('=') {
            if let Some(section) = config.sections.get_mut(&current_section) {
                section.insert(key.trim().to_string(), value.trim().to_string());
            }
        }
    }
    config
}
