# ============================================================
#  AJEEB LANGUAGE CONFIGURATION STANDARD — ajeeb.das
#  Single-source truth for project metadata, dependency
#  resolution, and cross-language FFI bridging.
# ============================================================

[module]
name = "AlphaRobotMPC"
version = "1.0.0"
syntax_style = "TypeScript"

# Enable foreign-language runtimes
[compatibility]
python_ai_core = "enabled"
cpp_physics_engine = "enabled"

# External package pins
[dependencies]
torch = "2.1"
tokio_network = "1.0"

# Runtime tuning
[runtime]
max_threads = "8"
network_port = "8080"
log_level = "info"
