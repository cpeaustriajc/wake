use std::collections::HashMap;
use std::env;
use std::path::Path;

use serde_json::Value;

const PROJECT_PLACEHOLDER: &str = "/work/project";
const PATH_PLACEHOLDER: &str = "/placeholder/path";
const HOME_PLACEHOLDER: &str = "/home/user";
const UUID_LENGTH: usize = 36;
const INIT_SUBTYPE: &str = "init";

pub(crate) struct Redactor {
    folder: String,
    homes: Vec<String>,
    forbidden: Vec<String>,
    uuids: HashMap<String, usize>,
    message_ids: HashMap<String, usize>,
}

fn is_uuid(text: &str) -> bool {
    text.len() == UUID_LENGTH
        && text.char_indices().all(|(index, character)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                character == '-'
            } else {
                character.is_ascii_hexdigit()
            }
        })
}

impl Redactor {
    pub(crate) fn new(folder: &Path) -> Self {
        let folder = folder.to_string_lossy().into_owned();
        let mut forbidden = vec![folder.clone()];
        let mut homes = Vec::new();
        if let Some(home) = env::var_os("HOME") {
            let home = home.to_string_lossy().into_owned();
            if home.len() > 1 {
                homes.push(home.clone());
                forbidden.push(home);
            }
        }
        if let Some(user) = env::var_os("USER") {
            let user = user.to_string_lossy().into_owned();
            if user.len() > 2 {
                forbidden.push(user);
            }
        }
        Self {
            folder,
            homes,
            forbidden,
            uuids: HashMap::new(),
            message_ids: HashMap::new(),
        }
    }

    pub(crate) fn line(&mut self, raw: &str) -> Result<String, String> {
        let mut value: Value = serde_json::from_str(raw).map_err(|error| error.to_string())?;
        let is_init = value.get("subtype").and_then(Value::as_str) == Some(INIT_SUBTYPE);
        if is_init && let Some(object) = value.as_object_mut() {
            for field in object.values_mut() {
                if field.is_array() {
                    *field = Value::Array(Vec::new());
                }
            }
        }
        self.walk(&mut value);
        serde_json::to_string(&value).map_err(|error| error.to_string())
    }

    fn walk(&mut self, value: &mut Value) {
        match value {
            Value::String(text) => *text = self.text(text),
            Value::Array(items) => items.iter_mut().for_each(|item| self.walk(item)),
            Value::Object(fields) => fields.values_mut().for_each(|field| self.walk(field)),
            Value::Null | Value::Bool(_) | Value::Number(_) => {}
        }
    }

    fn text(&mut self, text: &str) -> String {
        if is_uuid(text) {
            return self.uuid(text);
        }
        if text.starts_with("msg_") {
            let next = self.message_ids.len() + 1;
            let number = *self.message_ids.entry(text.to_owned()).or_insert(next);
            return format!("msg_placeholder{number}");
        }
        if let Some(rest) = text.strip_prefix(&self.folder) {
            return format!("{PROJECT_PLACEHOLDER}{rest}");
        }
        if text.starts_with('/') && !text.contains(' ') && text[1..].contains('/') {
            return PATH_PLACEHOLDER.to_owned();
        }
        self.embedded(text)
    }

    fn embedded(&mut self, text: &str) -> String {
        let mut text = text.replace(&self.folder, PROJECT_PLACEHOLDER);
        text = text.replace(&self.folder.replace('/', "-"), "-work-project");
        for home in &self.homes {
            text = text.replace(home.as_str(), HOME_PLACEHOLDER);
        }
        let mut redacted = String::with_capacity(text.len());
        let mut rest = text.as_str();
        while let Some(character) = rest.chars().next() {
            if let Some(candidate) = rest
                .get(..UUID_LENGTH)
                .filter(|candidate| is_uuid(candidate))
            {
                redacted.push_str(&self.uuid(candidate));
                rest = &rest[UUID_LENGTH..];
            } else {
                redacted.push(character);
                rest = &rest[character.len_utf8()..];
            }
        }
        redacted
    }

    fn uuid(&mut self, text: &str) -> String {
        let next = self.uuids.len() + 1;
        let number = *self.uuids.entry(text.to_owned()).or_insert(next);
        format!("00000000-0000-0000-0000-{number:012}")
    }

    pub(crate) fn check(&self, text: &str) -> Result<(), String> {
        for needle in &self.forbidden {
            if let Some(at) = text.find(needle.as_str()) {
                let start = text[..at]
                    .char_indices()
                    .rev()
                    .nth(60)
                    .map_or(0, |(i, _)| i);
                let end = text.len().min(at + needle.len() + 60);
                let context = text.get(start..end).unwrap_or_default();
                return Err(format!(
                    "recorded output still contains `{needle}` in `{context}`"
                ));
            }
        }
        if text.contains('@') {
            return Err("recorded output contains an `@`, which may be an email".to_owned());
        }
        Ok(())
    }
}
