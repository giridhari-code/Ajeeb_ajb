[package]
name = "ajeeb-compiler"
version = "0.1.0"
author = "Ajeeb Team"
description = "The Ajeeb self-hosting compiler"

[dependencies]
ajeeb-std = "0.1.0"

[runtime]
max_threads = "8"
log_level = "info"

[compiler]
target = "native"
entry = "ajeebc/compiler/main.ajb"
output = "build/"
runtime = "runtime/ajeeb_runtime.c"
