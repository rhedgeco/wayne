> ⚠️ __This repo is under active development and not ready to be used__

![Wayne](assets/readme/wayne-banner.png)

`wayne` is an experimental implementation of the wayland server protocol entirely in rust. It does not attempt to mirror the `libwayland` api, but rather re-imagine the api in a way that works best with rusty patterns and norms. This also means a departure from the built-in `libwayland` event loop, providing a more versitile _"bring your own"_ event loop architecture instead.

### Reference
Wayland documentation and reference can be found in a few places:

- [Wayland Website](https://wayland.freedesktop.org/) - The main wayland website
- [Wayland Explorer](https://wayland.app/protocols/) - A pretty wayland protocol explorer
- [Wayland Book](https://wayland-book.com/) - Explaining the wayland protocol

### License

Licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Your contributions

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
