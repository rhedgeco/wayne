//! XML Elements following the https://wayland.freedesktop.org specification
//!
//! Documentation for each struct and field is adapted directly from the spec definition.
//! Some parts have been removed or modified for brevity and clarity of the rust code.
//! See the spec for more information on the xml specifics.

/// Human-readable documentation for an element.
pub struct Description {
    /// A short (should be half a line at most) description of the documented element.
    pub summary: Option<String>,
    /// The free-form body text.
    /// May contain formatted text, including paragraphs and bulleted lists.
    pub body: Option<String>,
}

impl Description {
    /// Creates a new empty description.
    pub fn new() -> Self {
        Self {
            summary: None,
            body: None,
        }
    }

    /// Builder function that sets the summary.
    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    /// Builder function that sets the body text.
    pub fn with_body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }
}

/// The root element in a Wayland protocol XML file.
pub struct Protocol {
    /// The name of the protocol (a.k.a protocol extension).
    pub name: String,
    /// Copyright and license notices for the protocol.
    pub copyright: Option<String>,
    /// Documents the intended purpose of the protocol.
    pub description: Option<Description>,
    /// The interfaces that make up the protocol.
    pub interfaces: Vec<Interface>,
}

impl Protocol {
    /// Creates a new protocol with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            copyright: None,
            description: None,
            interfaces: Vec::new(),
        }
    }

    /// Builder function that sets the copyright notice.
    pub fn with_copyright(mut self, copyright: impl Into<String>) -> Self {
        self.copyright = Some(copyright.into());
        self
    }

    /// Builder function that sets the description.
    pub fn with_description(mut self, description: Description) -> Self {
        self.description = Some(description);
        self
    }

    /// Builder function that adds an interface.
    pub fn with_interface(mut self, interface: Interface) -> Self {
        self.interfaces.push(interface);
        self
    }
}

/// A collection of the requests and events that form the interface, along with any enumerations.
/// These all belong to the namespace of the interface.
pub struct Interface {
    /// The name of the interface. Must be unique in the protocol.
    pub name: String,
    /// The interface's latest version number.
    /// An interface defines all versions from 1 to this value inclusive.
    pub version: u32,
    /// The interface is frozen and forever stuck at version 1.
    pub frozen: bool,
    /// Describes the purpose and the general usage of the interface.
    pub description: Option<Description>,
    /// The requests defined by the interface (messages from client to server).
    pub requests: Vec<Request>,
    /// The events defined by the interface (messages from server to client).
    pub events: Vec<Event>,
    /// The enumerations defined by the interface.
    pub enums: Vec<Enum>,
}

impl Interface {
    /// Creates a new interface with the given name at version 1.
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

    /// Builder function that sets the version.
    pub fn version(mut self, version: u32) -> Self {
        self.version = version;
        self
    }

    /// Builder function that marks the interface as frozen.
    pub fn frozen(mut self) -> Self {
        self.frozen = true;
        self
    }

    /// Builder function that sets the description.
    pub fn with_description(mut self, description: Description) -> Self {
        self.description = Some(description);
        self
    }

    /// Builder function that adds a request.
    pub fn with_request(mut self, request: Request) -> Self {
        self.requests.push(request);
        self
    }

    /// Builder function that adds an event.
    pub fn with_event(mut self, event: Event) -> Self {
        self.events.push(event);
        self
    }

    /// Builder function that adds an enum.
    pub fn with_enum(mut self, variant: Enum) -> Self {
        self.enums.push(variant);
        self
    }
}

/// A request: a message from a client to a server.
/// Requests are always associated with a specific protocol object.
///
/// Requests are automatically assigned opcodes in the order they appear inside the interface element.
pub struct Request {
    /// The name of the request.
    /// Must be unique within all requests and events in the containing interface.
    pub name: String,
    /// The request's arguments. The order defines the order on the wire.
    /// All declared arguments are mandatory.
    pub args: Vec<Arg>,
    /// The request is a destructor: it destroys the protocol object it is sent
    /// on.
    pub destructor: bool,
    /// The request was added in this interface version.
    /// If absent, version 1 is assumed.
    pub since: Option<u32>,
    /// The request was deprecated in this interface version and above.
    /// Must be greater than the value of `since`.
    pub deprecated_since: Option<u32>,
    /// Documents the request.
    pub description: Option<Description>,
}

impl Request {
    /// Creates a new request with the given name.
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

    /// Builder function that adds an argument.
    pub fn with_arg(mut self, arg: Arg) -> Self {
        self.args.push(arg);
        self
    }

    /// Builder function that marks the request as a destructor.
    pub fn destructor(mut self) -> Self {
        self.destructor = true;
        self
    }

    /// Builder function that sets the version the request was added in.
    pub fn since(mut self, since: u32) -> Self {
        self.since = Some(since);
        self
    }

    /// Builder function that sets the version the request was deprecated in.
    pub fn deprecated_since(mut self, deprecated_since: u32) -> Self {
        self.deprecated_since = Some(deprecated_since);
        self
    }

    /// Builder function that sets the description.
    pub fn with_description(mut self, description: Description) -> Self {
        self.description = Some(description);
        self
    }
}

/// An event: a message from a server to a client.
/// Events are always associated with a specific protocol object.
///
/// Events are automatically assigned opcodes in the order they appear inside the interface element.
pub struct Event {
    /// The name of the event.
    /// Must be unique within all requests and events in the containing interface.
    pub name: String,
    /// The event's arguments. The order defines the order on the wire.
    /// All declared arguments are mandatory.
    pub args: Vec<Arg>,
    /// The event is a destructor: it destroys the protocol object it is sent on.
    pub destructor: bool,
    /// The event was added in this interface version.
    /// If absent, version 1 is assumed.
    pub since: Option<u32>,
    /// The event was deprecated in this interface version and above.
    pub deprecated_since: Option<u32>,
    /// Documents the event.
    pub description: Option<Description>,
}

impl Event {
    /// Creates a new event with the given name.
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

    /// Builder function that adds an argument.
    pub fn with_arg(mut self, arg: Arg) -> Self {
        self.args.push(arg);
        self
    }

    /// Builder function that marks the event as a destructor.
    pub fn destructor(mut self) -> Self {
        self.destructor = true;
        self
    }

    /// Builder function that sets the version the event was added in.
    pub fn since(mut self, since: u32) -> Self {
        self.since = Some(since);
        self
    }

    /// Builder function that sets the version the event was deprecated in.
    pub fn deprecated_since(mut self, deprecated_since: u32) -> Self {
        self.deprecated_since = Some(deprecated_since);
        self
    }

    /// Builder function that sets the description.
    pub fn with_description(mut self, description: Description) -> Self {
        self.description = Some(description);
        self
    }
}

/// One argument of a request or an event. All arguments are typed.
pub struct Arg {
    /// The name of the argument.
    /// Must be unique within all the arguments of the parent element.
    pub name: String,
    /// The type of the argument datum.
    pub ty: ArgType,
    /// If given, the name of the interface the existing or new object must have.
    /// Only used when `ty` is `Object` or `NewId`.
    pub interface: Option<String>,
    /// If specified, the argument value should come from the named enum.
    /// The name may be qualified with an interface name using a period.
    pub enumeration: Option<String>,
    /// Whether the argument value can be null on send.
    /// Only used when `ty` is `String` or `Object`.
    pub allow_null: bool,
    /// A short (half a line at most) description.
    /// Should usually not be used if a description is used.
    pub summary: Option<String>,
    /// Documents the argument.
    pub description: Option<Description>,
}

impl Arg {
    /// Creates a new argument with the given name and type.
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

    /// Builder function that sets the interface name.
    pub fn with_interface(mut self, interface: impl Into<String>) -> Self {
        self.interface = Some(interface.into());
        self
    }

    /// Builder function that sets the enum the value comes from.
    pub fn with_enumeration(mut self, enumeration: impl Into<String>) -> Self {
        self.enumeration = Some(enumeration.into());
        self
    }

    /// Builder function that allows the argument value to be null.
    pub fn allow_null(mut self) -> Self {
        self.allow_null = true;
        self
    }

    /// Builder function that sets the summary.
    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    /// Builder function that sets the description.
    pub fn with_description(mut self, description: Description) -> Self {
        self.description = Some(description);
        self
    }
}

/// The type of an argument datum.
pub enum ArgType {
    /// 32-bit signed integer.
    Int,
    /// 32-bit unsigned integer.
    Uint,
    /// Signed 24.8-bit fixed-point value.
    Fixed,
    /// UTF-8 encoded string value, NUL byte terminated.
    String,
    /// Reference to an existing protocol object.
    Object,
    /// Creates a new protocol object.
    /// A message may have at most one `new_id` argument.
    NewId,
    /// A byte array of arbitrary data.
    Array,
    /// A file descriptor. Must be open and valid on send.
    Fd,
}

/// An enumeration of integer values.
/// Enumerations give names to arbitrary integer constants.
pub struct Enum {
    /// The name of the enumeration.
    /// Must be unique within all enumerations in the containing interface.
    /// Used as the namespace for its entries.
    pub name: String,
    /// Whether this enumeration is a bitfield.
    pub bitfield: bool,
    /// The enumeration was added in this interface version.
    /// If absent, version 1 is assumed.
    pub since: Option<u32>,
    /// Describes the enumeration.
    pub description: Option<Description>,
    /// The values that belong to the enumeration.
    pub entries: Vec<Entry>,
}

impl Enum {
    /// Creates a new enumeration with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            bitfield: false,
            since: None,
            description: None,
            entries: Vec::new(),
        }
    }

    /// Builder function that marks the enumeration as a bitfield.
    pub fn bitfield(mut self) -> Self {
        self.bitfield = true;
        self
    }

    /// Builder function that sets the version the enumeration was added in.
    pub fn since(mut self, since: u32) -> Self {
        self.since = Some(since);
        self
    }

    /// Builder function that sets the description.
    pub fn with_description(mut self, description: Description) -> Self {
        self.description = Some(description);
        self
    }

    /// Builder function that adds an entry.
    pub fn with_entry(mut self, entry: Entry) -> Self {
        self.entries.push(entry);
        self
    }
}

/// A name for an integer constant, part of the set of values of the containing enumeration.
pub struct Entry {
    /// The name of a value in an enumeration.
    /// Must be unique within all entries in the containing enum.
    pub name: String,
    /// The integer value for this entry.
    pub value: u32,
    /// A short (half a line at most) description.
    /// Should usually not be used if a description is used.
    pub summary: Option<String>,
    /// The value was added in this interface version.
    /// If absent, version 1 is assumed.
    pub since: Option<u32>,
    /// The value was removed in this interface version and above.
    pub deprecated_since: Option<u32>,
    /// Documents the entry.
    pub description: Option<Description>,
}

impl Entry {
    /// Creates a new entry with the given name and value.
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

    /// Builder function that sets the summary.
    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    /// Builder function that sets the version the entry was added in.
    pub fn since(mut self, since: u32) -> Self {
        self.since = Some(since);
        self
    }

    /// Builder function that sets the version the entry was deprecated in.
    pub fn deprecated_since(mut self, deprecated_since: u32) -> Self {
        self.deprecated_since = Some(deprecated_since);
        self
    }

    /// Builder function that sets the description.
    pub fn with_description(mut self, description: Description) -> Self {
        self.description = Some(description);
        self
    }
}
