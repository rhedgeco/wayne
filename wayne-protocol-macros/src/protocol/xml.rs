use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename = "protocol")]
pub struct Protocol {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "interface")]
    pub interfaces: Vec<Interface>,
}

#[derive(Debug, Default, Deserialize)]
pub struct Description {
    #[serde(rename = "@summary")]
    pub summary: String,
    #[serde(rename = "$text")]
    pub text: Option<String>,
}

impl Description {
    pub fn lines(&self) -> impl Iterator<Item = &str> {
        match &self.text {
            Some(text) => text.lines(),
            None => self.summary.lines(),
        }
        .map(str::trim)
    }
}

#[derive(Debug, Deserialize)]
pub struct Interface {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@version")]
    pub version: u32,
    #[serde(rename = "description")]
    pub description: Description,
    #[serde(default, rename = "request")]
    pub requests: Vec<Request>,
    #[serde(default, rename = "event")]
    pub events: Vec<Event>,
    #[serde(default, rename = "enum")]
    pub enums: Vec<Enum>,
}

#[derive(Debug, Deserialize)]
pub struct Request {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "description")]
    pub description: Description,
    #[serde(default, rename = "arg")]
    pub args: Vec<Arg>,
}

#[derive(Debug, Deserialize)]
pub struct Arg {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@type")]
    pub ty: ArgType,
    #[serde(rename = "@interface")]
    pub interface: Option<String>,
    #[serde(rename = "@enum")]
    pub enum_kind: Option<String>,
    // #[serde(
    //     default,
    //     rename = "@allow-null",
    //     deserialize_with = "utils::parse_bool"
    // )]
    // pub allow_null: bool,
    #[serde(rename = "@summary")]
    pub summary: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArgType {
    Int,
    Uint,
    Fixed,
    String,
    Object,
    NewId,
    Array,
    Fd,
}

#[derive(Debug, Deserialize)]
pub struct Event {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "description")]
    pub description: Description,
    #[serde(default, rename = "arg")]
    pub args: Vec<Arg>,
}

#[derive(Debug, Deserialize)]
pub struct Enum {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(default, rename = "description")]
    pub description: Description,
    #[serde(rename = "entry")]
    pub entries: Vec<Entry>,
}

#[derive(Debug, Deserialize)]
pub struct Entry {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@value", deserialize_with = "utils::parse_hex")]
    pub value: u32,
    #[serde(default, rename = "@summary")]
    pub summary: String,
}

mod utils {
    use serde::{Deserialize, Deserializer};

    // pub fn parse_bool<'de, D: Deserializer<'de>>(deserializer: D) -> Result<bool, D::Error> {
    //     let string = String::deserialize(deserializer)?;
    //     match string.as_str() {
    //         "true" => Ok(true),
    //         _ => Ok(false),
    //     }
    // }

    pub fn parse_hex<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u32, D::Error> {
        let string = String::deserialize(deserializer)?;
        match string.starts_with("0x") {
            false => string
                .parse::<u32>()
                .map_err(|err| serde::de::Error::custom(err.to_string())),
            true => u32::from_str_radix(&string[2..], 16)
                .map_err(|err| serde::de::Error::custom(err.to_string())),
        }
    }
}
