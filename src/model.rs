use std::borrow::Cow;

pub type Str<'a> = Cow<'a, str>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value<'a> {
    Str(Str<'a>),
    Map(Vec<Entry<'a>>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry<'a> {
    pub key: Str<'a>,
    pub value: Value<'a>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Section<'a> {
    pub name: Str<'a>,
    pub entries: Vec<Entry<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Document<'a> {
    pub meta: Vec<(Str<'a>, Str<'a>)>,
    pub sections: Vec<Section<'a>>,
}

#[inline]
fn find<'s, 'a>(entries: &'s [Entry<'a>], key: &str) -> Option<&'s Value<'a>> {
    entries.iter().find(|e| e.key == key).map(|e| &e.value)
}

fn own(s: Str<'_>) -> Str<'static> {
    Cow::Owned(s.into_owned())
}


impl<'a> Value<'a> {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(&**s),
            Value::Map(_) => None,
        }
    }

    pub fn as_map(&self) -> Option<&[Entry<'a>]> {
        match self {
            Value::Map(m) => Some(m.as_slice()),
            Value::Str(_) => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self.as_str()? {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        }
    }

    pub fn parse<T: std::str::FromStr>(&self) -> Option<T> {
        self.as_str()?.parse().ok()
    }

    pub fn get(&self, key: &str) -> Option<&Value<'a>> {
        find(self.as_map()?, key)
    }

    pub fn into_owned(self) -> Value<'static> {
        match self {
            Value::Str(s) => Value::Str(own(s)),
            Value::Map(m) => Value::Map(m.into_iter().map(Entry::into_owned).collect()),
        }
    }
}

impl<'a> From<&'a str> for Value<'a> {
    fn from(s: &'a str) -> Self {
        Value::Str(Cow::Borrowed(s))
    }
}

impl<'a> From<String> for Value<'a> {
    fn from(s: String) -> Self {
        Value::Str(Cow::Owned(s))
    }
}

impl<'a> From<Cow<'a, str>> for Value<'a> {
    fn from(s: Cow<'a, str>) -> Self {
        Value::Str(s)
    }
}

impl<'a> From<Vec<Entry<'a>>> for Value<'a> {
    fn from(m: Vec<Entry<'a>>) -> Self {
        Value::Map(m)
    }
}


impl<'a> Entry<'a> {
    pub fn new(key: impl Into<Str<'a>>, value: impl Into<Value<'a>>) -> Self {
        Entry { key: key.into(), value: value.into() }
    }

    pub fn into_owned(self) -> Entry<'static> {
        Entry { key: own(self.key), value: self.value.into_owned() }
    }
}


impl<'a> Section<'a> {
    pub fn new(name: impl Into<Str<'a>>) -> Self {
        Section { name: name.into(), entries: Vec::new() }
    }

    pub fn get(&self, key: &str) -> Option<&Value<'a>> {
        find(&self.entries, key)
    }

    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(|v| v.as_str())
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut Value<'a>> {
        self.entries.iter_mut().find(|e| e.key == key).map(|e| &mut e.value)
    }

    pub fn set(&mut self, key: impl Into<Str<'a>>, value: impl Into<Value<'a>>) {
        let key = key.into();
        let value = value.into();
        if let Some(i) = self.entries.iter().position(|e| e.key == key) {
            self.entries[i].value = value;
        } else {
            self.entries.push(Entry { key, value });
        }
    }

    pub fn push(&mut self, key: impl Into<Str<'a>>, value: impl Into<Value<'a>>) {
        self.entries.push(Entry::new(key, value));
    }

    pub fn remove(&mut self, key: &str) -> Option<Value<'a>> {
        let i = self.entries.iter().position(|e| e.key == key)?;
        Some(self.entries.remove(i).value)
    }

    pub fn into_owned(self) -> Section<'static> {
        Section {
            name: own(self.name),
            entries: self.entries.into_iter().map(Entry::into_owned).collect(),
        }
    }
}


impl<'a> Document<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn section(&self, name: &str) -> Option<&Section<'a>> {
        self.sections.iter().find(|s| s.name == name)
    }

    pub fn section_mut(&mut self, name: &str) -> Option<&mut Section<'a>> {
        self.sections.iter_mut().find(|s| s.name == name)
    }

    pub fn section_or_insert(&mut self, name: impl Into<Str<'a>>) -> &mut Section<'a> {
        let name = name.into();
        let idx = match self.sections.iter().position(|s| s.name == name) {
            Some(i) => i,
            None => {
                self.sections.push(Section::new(name));
                self.sections.len() - 1
            }
        };
        &mut self.sections[idx]
    }

    pub fn get(&self, section: &str, key: &str) -> Option<&Value<'a>> {
        self.section(section)?.get(key)
    }

    pub fn into_owned(self) -> Document<'static> {
        Document {
            meta: self.meta.into_iter().map(|(k, v)| (own(k), own(v))).collect(),
            sections: self.sections.into_iter().map(Section::into_owned).collect(),
        }
    }
}
