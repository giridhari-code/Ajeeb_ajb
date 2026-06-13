[package]
name = "ajeeb-web"
version = "1.0.0"
author = "Ajeeb Ecosystem"
description = "Production-style web framework with routing, middleware, DI, and more"
registry = ""

[dependencies]
ajeeb-json = "^1.0.0"
ajeeb-log = "^1.0.0"
ajeeb-http = "^1.0.0"
ajeeb-db = "^1.0.0"

[compiler]
target = "native"
output = "build/"
runtime = "runtime/ajeeb_runtime.c"
