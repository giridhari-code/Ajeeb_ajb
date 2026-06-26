[package]
name = "testp"
version = "0.1.0"
author = ""
description = ""
registry = ""

[dependencies]

[profile.dev]
opt-level = "0"
debug = "true"

[profile.release]
opt-level = "3"
debug = "false"
lto = "true"
