# LibOSDP for Rust

This crate provides safe wrappers for accessing the C library [LibOSDP][1].
LibOSDP the most popular open source library for creating Open Supervised Device
Protocol (OSDP) devices. For more information about the library or OSDP in
general, see [libosdp.sidcha.dev][4].

The documentation for this crate can be found [here][5].

## Features

  - Production ready implementation with active users and contributors
  - Supports secure channel communication (AES-128)
  - Can be used to setup a PD or CP mode of operation
  - Most OSDP specified commands and replies are supported (see [doc][3])

## Usage

To add libosdp to your rust project, do:

```
cargo add libosdp
```

### Control Panel:

A simplified CP implementation:

```rust
let pd_info = vec! [ PdInfo::new(...), ... ];
let mut cp = ControlPanel::new(&mut pd_info)?;
cp.set_event_callback(|pd, event| {
    println!("Received event from {pd}: {:?}", event);
});
loop {
    cp.refresh();
    cp.send_command(0, OsdpCommand::new(...));
}
```

See [examples][2] for a working implementation.

### Peripheral Device:

A simplified PD implementation:

```rust
let pd_info = PdInfo::new(...);
let mut pd = PeripheralDevice::new(&mut pd_info)?;
pd.set_command_callback(|cmd| {
    println!("Received command {:?}", cmd);
});
loop {
    pd.refresh();
    cp.notify_event(OsdpEvent::new(...));
}
```

See [examples][2] for a working implementation.

[1]: https://github.cobm/goToMain/liosdp
[2]: https://github.com/goToMain/libosdp-rs/tree/master/libosdp/examples
[3]: https://libosdp.sidcha.dev/protocol/commands-and-replies
[4]: https://libosdp.sidcha.dev/
[5]: https://docs.rs/libosdp