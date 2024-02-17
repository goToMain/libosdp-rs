# LibOSDP-rs

This project hosts cargo crates that add rust support for [LibOSDP][1] and other
OSDP related tools. You can also take a look at the [documentation][2] for more
details.

Repo structure:
- `libosdp-sys` - Low level rust `-sys` crate for the C library.
- `libosdp` - Safe wrapper around `libosdp-sys` to be consumed by rust projects.
- `osdpctl` - A tool to create and manage OSDP devices.
- `scripts` - Tools for developers working on this project.

[1]: https://github.com/goToMain/libosdp
[2]: https://libosdp.sidcha.dev/
