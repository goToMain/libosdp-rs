# libosdp-sys

This crate hosts bindgen generated `-sys` bindings to [goToMain/libosdp][1]. It
tracks LibOSDP releases and uses the same version numbers to make it easy to
determine the underlying LibOSDP version.

This crate is not intended to be directly consumed. Please take a look at
[libosdp][2] (see doc [here][3]) if you intend to use LibOSDP in your project.

[1]: https://github.com/goToMain/libosdp
[2]: https://crates.io/crates/libosdp
[3]: https://docs.rs/libosdp