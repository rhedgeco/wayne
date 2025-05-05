use std::os::fd::OwnedFd;

use wayne_protocol::{
    parse::{self, MapExt, Parser, ThenExt},
    types::ObjId,
};

fn main() {}

pub enum MyInterfaceRequest {
    Request0(Request0),
    Request1(Request1),
}

impl MyInterfaceRequest {
    pub fn parser(opcode: u16) -> Option<impl Parser<Output = Self>> {
        match opcode {
            0 => Some(MyInterfaceParser::Request0(Request0::parser())),
            1 => Some(MyInterfaceParser::Request1(Request1::parser())),
            _ => None,
        }
    }
}

pub enum MyInterfaceParser<P0, P1>
where
    P0: Parser<Output = Request0>,
    P1: Parser<Output = Request1>,
{
    Request0(P0),
    Request1(P1),
}

impl<P0, P1> Parser for MyInterfaceParser<P0, P1>
where
    P0: Parser<Output = Request0>,
    P1: Parser<Output = Request1>,
{
    type Output = MyInterfaceRequest;

    fn parse(self, buffer: impl parse::Buffer) -> Result<Self::Output, Self> {
        match self {
            MyInterfaceParser::Request0(parser) => parser
                .parse(buffer)
                .map(|request| MyInterfaceRequest::Request0(request))
                .map_err(|parser| MyInterfaceParser::Request0(parser)),
            MyInterfaceParser::Request1(parser) => parser
                .parse(buffer)
                .map(|request| MyInterfaceRequest::Request1(request))
                .map_err(|parser| MyInterfaceParser::Request1(parser)),
        }
    }
}

pub struct Request0 {
    float: f32,
    string: String,
    array: Box<[u8]>,
    fd: OwnedFd,
}

impl Request0 {
    pub fn parser() -> impl Parser<Output = Self> {
        parse::f32().then(move |float| {
            parse::string().then(move |string| {
                parse::array().then(move |array| {
                    parse::fd().map(move |fd| Self {
                        float,
                        string,
                        array,
                        fd,
                    })
                })
            })
        })
    }
}

pub struct Request1 {
    int: i32,
    uint: u32,
    id: ObjId<MyInterfaceRequest>,
}

impl Request1 {
    pub fn parser() -> impl Parser<Output = Self> {
        parse::i32().then(move |int| {
            parse::u32().then(move |uint| parse::obj_id().map(move |id| Self { int, uint, id }))
        })
    }
}
