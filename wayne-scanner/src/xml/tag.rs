use std::{collections::HashMap, io::BufRead};

use quick_xml::{
    Reader,
    events::{BytesStart, Event},
};

use super::{XmlError, error::XmlResult};

pub struct XmlTag {
    name: String,
    attrs: HashMap<String, String>,
    inner_tags: HashMap<String, Vec<XmlTag>>,
    inner_text: String,
}

impl XmlTag {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn inner_text(&self) -> &str {
        &self.inner_text
    }

    pub fn inner_tags(&self, name: impl AsRef<str>) -> Option<&[XmlTag]> {
        self.inner_tags.get(name.as_ref()).map(|v| v.as_slice())
    }

    pub fn get_attr(&self, name: impl AsRef<str>) -> Option<&str> {
        self.attrs.get(name.as_ref()).map(|s| s.as_str())
    }

    pub fn empty(start: &BytesStart) -> Self {
        Self {
            name: String::from_utf8_lossy(start.name().as_ref()).to_string(),
            attrs: start
                .attributes()
                .filter_map(|r| r.ok())
                .map(|attr| {
                    (
                        String::from_utf8_lossy(attr.key.as_ref()).to_string(),
                        String::from_utf8_lossy(&attr.value).to_string(),
                    )
                })
                .collect(),
            inner_tags: HashMap::new(),
            inner_text: String::new(),
        }
    }

    pub fn build<R: BufRead>(start: &BytesStart, reader: &mut Reader<R>) -> XmlResult<Self> {
        let mut tag = Self::empty(start);

        loop {
            match reader
                .read_event_into(&mut Vec::new())
                .map_err(|e| XmlError::new(e.into(), reader.error_position()))?
            {
                Event::Text(text) => tag.inner_text.push_str(&String::from_utf8_lossy(&text)),
                Event::Start(start) => {
                    let inner_tag = Self::build(&start, reader)?;
                    tag.insert_inner(inner_tag);
                }
                Event::Empty(start) => {
                    let inner_tag = Self::empty(&start);
                    tag.insert_inner(inner_tag);
                }
                Event::End(end) => match end.name().as_ref() == start.name().as_ref() {
                    true => break,
                    false => {
                        return Err(XmlError::new(
                            anyhow::Error::msg("unexpected closing tag"),
                            reader.buffer_position(),
                        ));
                    }
                },
                Event::Eof => {
                    return Err(XmlError::new(
                        anyhow::Error::msg("unexpected EOF"),
                        reader.buffer_position(),
                    ));
                }
                _ => {} // do nothing
            }
        }

        Ok(tag)
    }

    fn insert_inner(&mut self, tag: XmlTag) {
        use std::collections::hash_map::Entry as E;
        match self.inner_tags.entry(tag.name().into()) {
            E::Occupied(mut entry) => entry.get_mut().push(tag),
            E::Vacant(entry) => {
                entry.insert(vec![tag]);
            }
        }
    }
}
