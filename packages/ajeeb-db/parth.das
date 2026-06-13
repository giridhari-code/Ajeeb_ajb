[package]
name = "ajeeb-db"
version = "1.0.0"
author = "Ajeeb Ecosystem"
description = "Database abstraction layer with SQLite support"
registry = ""

[dependencies]
ajeeb-json = "^1.0.0"
ajeeb-log = "^1.0.0"

[runtime]
sqlite = "bundled"

[compiler]
target = "native"
output = "build/"
runtime = "runtime/ajeeb_runtime.c"
