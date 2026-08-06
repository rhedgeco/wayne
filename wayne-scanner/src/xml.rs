/// Human-readable documentation for an element.
pub struct Description {
    /// The short one-line `summary` attribute.
    pub summary: Option<String>,
    /// The free-form body text.
    pub body: Option<String>,
}

impl Description {
    pub fn new() -> Self {
        Self {
            summary: None,
            body: None,
        }
    }

    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    pub fn with_body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }
}

pub struct Protocol {
    pub name: String,
    pub copyright: Option<String>,
    pub description: Option<Description>,
    pub interfaces: Vec<Interface>,
}

impl Protocol {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            copyright: None,
            description: None,
            interfaces: Vec::new(),
        }
    }

    pub fn with_copyright(mut self, copyright: impl Into<String>) -> Self {
        self.copyright = Some(copyright.into());
        self
    }

    pub fn with_description(mut self, description: Description) -> Self {
        self.description = Some(description);
        self
    }

    pub fn with_interface(mut self, interface: Interface) -> Self {
        self.interfaces.push(interface);
        self
    }
}

/// An interface: a named collection of requests, events, and enums.
pub struct Interface {
    pub name: String,
    pub version: u32,
    pub frozen: bool,
    pub description: Option<Description>,
    pub requests: Vec<Request>,
    pub events: Vec<Event>,
    pub enums: Vec<Enum>,
}

impl Interface {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: 1,
            frozen: false,
            description: None,
            requests: Vec::new(),
            events: Vec::new(),
            enums: Vec::new(),
        }
    }

    pub fn version(mut self, version: u32) -> Self {
        self.version = version;
        self
    }

    pub fn frozen(mut self) -> Self {
        self.frozen = true;
        self
    }

    pub fn with_description(mut self, description: Description) -> Self {
        self.description = Some(description);
        self
    }

    pub fn with_request(mut self, request: Request) -> Self {
        self.requests.push(request);
        self
    }

    pub fn with_event(mut self, event: Event) -> Self {
        self.events.push(event);
        self
    }

    pub fn with_enum(mut self, variant: Enum) -> Self {
        self.enums.push(variant);
        self
    }
}

/// A request: a message sent from a client to the server.
pub struct Request {
    pub name: String,
    pub args: Vec<Arg>,
    pub destructor: bool,
    pub since: Option<u32>,
    pub deprecated_since: Option<u32>,
    pub description: Option<Description>,
}

impl Request {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            args: Vec::new(),
            destructor: false,
            since: None,
            deprecated_since: None,
            description: None,
        }
    }

    pub fn with_arg(mut self, arg: Arg) -> Self {
        self.args.push(arg);
        self
    }

    pub fn destructor(mut self) -> Self {
        self.destructor = true;
        self
    }

    pub fn since(mut self, since: u32) -> Self {
        self.since = Some(since);
        self
    }

    pub fn deprecated_since(mut self, deprecated_since: u32) -> Self {
        self.deprecated_since = Some(deprecated_since);
        self
    }

    pub fn with_description(mut self, description: Description) -> Self {
        self.description = Some(description);
        self
    }
}

/// An event: a message sent from the server to a client.
pub struct Event {
    pub name: String,
    pub args: Vec<Arg>,
    pub destructor: bool,
    pub since: Option<u32>,
    pub deprecated_since: Option<u32>,
    pub description: Option<Description>,
}

impl Event {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            args: Vec::new(),
            destructor: false,
            since: None,
            deprecated_since: None,
            description: None,
        }
    }

    pub fn with_arg(mut self, arg: Arg) -> Self {
        self.args.push(arg);
        self
    }

    pub fn destructor(mut self) -> Self {
        self.destructor = true;
        self
    }

    pub fn since(mut self, since: u32) -> Self {
        self.since = Some(since);
        self
    }

    pub fn deprecated_since(mut self, deprecated_since: u32) -> Self {
        self.deprecated_since = Some(deprecated_since);
        self
    }

    pub fn with_description(mut self, description: Description) -> Self {
        self.description = Some(description);
        self
    }
}

/// A single argument of a request or event.
pub struct Arg {
    pub name: String,
    pub ty: ArgType,
    pub interface: Option<String>,
    pub enumeration: Option<String>,
    pub allow_null: bool,
    pub summary: Option<String>,
    pub description: Option<Description>,
}

impl Arg {
    pub fn new(name: impl Into<String>, ty: ArgType) -> Self {
        Self {
            name: name.into(),
            ty,
            interface: None,
            enumeration: None,
            allow_null: false,
            summary: None,
            description: None,
        }
    }

    pub fn with_interface(mut self, interface: impl Into<String>) -> Self {
        self.interface = Some(interface.into());
        self
    }

    pub fn with_enumeration(mut self, enumeration: impl Into<String>) -> Self {
        self.enumeration = Some(enumeration.into());
        self
    }

    pub fn allow_null(mut self) -> Self {
        self.allow_null = true;
        self
    }

    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    pub fn with_description(mut self, description: Description) -> Self {
        self.description = Some(description);
        self
    }
}

/// The type of a request or event argument.
pub enum ArgType {
    /// 32-bit signed integer.
    Int,
    /// 32-bit unsigned integer.
    Uint,
    /// Signed 24.8-bit fixed-point value.
    Fixed,
    /// UTF-8 encoded, NUL-terminated string.
    String,
    /// Reference to an existing object.
    Object,
    /// Creates a new object.
    NewId,
    /// A byte array of arbitrary data.
    Array,
    /// A file descriptor.
    Fd,
}

/// An enumeration of named integer constants.
pub struct Enum {
    pub name: String,
    pub bitfield: bool,
    pub since: Option<u32>,
    pub description: Option<Description>,
    pub entries: Vec<Entry>,
}

impl Enum {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            bitfield: false,
            since: None,
            description: None,
            entries: Vec::new(),
        }
    }

    pub fn bitfield(mut self) -> Self {
        self.bitfield = true;
        self
    }

    pub fn since(mut self, since: u32) -> Self {
        self.since = Some(since);
        self
    }

    pub fn with_description(mut self, description: Description) -> Self {
        self.description = Some(description);
        self
    }

    pub fn with_entry(mut self, entry: Entry) -> Self {
        self.entries.push(entry);
        self
    }
}

/// A named constant within an [`Enum`].
pub struct Entry {
    pub name: String,
    pub value: u32,
    pub summary: Option<String>,
    pub since: Option<u32>,
    pub deprecated_since: Option<u32>,
    pub description: Option<Description>,
}

impl Entry {
    pub fn new(name: impl Into<String>, value: u32) -> Self {
        Self {
            name: name.into(),
            value,
            summary: None,
            since: None,
            deprecated_since: None,
            description: None,
        }
    }

    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    pub fn since(mut self, since: u32) -> Self {
        self.since = Some(since);
        self
    }

    pub fn deprecated_since(mut self, deprecated_since: u32) -> Self {
        self.deprecated_since = Some(deprecated_since);
        self
    }

    pub fn with_description(mut self, description: Description) -> Self {
        self.description = Some(description);
        self
    }
}
