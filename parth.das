[package]
name = "my-project"
version = "0.1.0"
author = ""
description = ""

[dependencies]

[runtime]
max_threads = "8"
log_level = "info"

[compiler]
target = "native"
output = "build/"
runtime = "runtime/ajeeb_runtime.c"
