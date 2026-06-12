[package]
name = "ajeeb-compiler"
version = "0.1.0"
author = "Ajeeb Team"
description = "The Ajeeb self-hosting compiler"

[dependencies]

[runtime]
max_threads = "8"
log_level = "info"

[compiler]
target = "native"
output = "build/"
runtime = "runtime/ajeeb_runtime.c"
