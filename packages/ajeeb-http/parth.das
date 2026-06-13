[package]
name = "ajeeb-http"
version = "1.0.0"
author = "Ajeeb Ecosystem"
description = "HTTP server with routing, request parsing, and response building"
registry = ""

[dependencies]
ajeeb-json = "^1.0.0"
ajeeb-log = "^1.0.0"

[compiler]
target = "native"
output = "build/"
runtime = "runtime/ajeeb_runtime.c"
