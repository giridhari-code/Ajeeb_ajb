use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::{c_char, c_void};

extern "C" {
    fn dlopen(filename: *const c_char, flags: i32) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlerror() -> *mut c_char;
}

const RTLD_NOW: i32 = 0x002;
const RTLD_LOCAL: i32 = 0x000;

/// Manages dynamically loaded shared libraries and their function pointers.
pub struct FfiRegistry {
    /// library_path → dlopen handle
    handles: HashMap<String, *mut c_void>,
    /// "lib_path::fn_name" → function pointer
    symbols: HashMap<String, *mut c_void>,
    /// library alias → library_path (from @import "lib" as alias)
    aliases: HashMap<String, String>,
}

impl FfiRegistry {
    pub fn new() -> Self {
        FfiRegistry {
            handles: HashMap::new(),
            symbols: HashMap::new(),
            aliases: HashMap::new(),
        }
    }

    /// Load a shared library via dlopen. Returns handle or error message.
    pub fn load_library(&mut self, path: &str) -> Result<*mut c_void, String> {
        if let Some(&handle) = self.handles.get(path) {
            return Ok(handle);
        }
        let c_path = CString::new(path).map_err(|e| format!("Galat library path: {}", e))?;
        let handle = unsafe { dlopen(c_path.as_ptr(), RTLD_NOW | RTLD_LOCAL) };
        if handle.is_null() {
            let err = unsafe { dlerror() };
            let msg = if err.is_null() {
                "dlopen returned NULL".to_string()
            } else {
                unsafe { std::ffi::CStr::from_ptr(err).to_string_lossy().to_string() }
            };
            return Err(format!("Library load nahi ho paayi '{}': {}", path, msg));
        }
        self.handles.insert(path.to_string(), handle);
        Ok(handle)
    }

    /// Register a library alias: mylib → /path/to/lib.so
    pub fn register_alias(&mut self, alias: &str, path: &str) {
        self.aliases.insert(alias.to_string(), path.to_string());
    }

    /// Resolve an alias to its actual path.
    pub fn resolve_alias(&self, alias: &str) -> Option<&str> {
        self.aliases.get(alias).map(|s| s.as_str())
    }

    /// Look up a symbol in a loaded library.
    pub fn lookup_symbol(&mut self, lib_path: &str, fn_name: &str) -> Result<*mut c_void, String> {
        let key = format!("{}::{}", lib_path, fn_name);
        if let Some(&ptr) = self.symbols.get(&key) {
            return Ok(ptr);
        }
        let handle = *self.handles.get(lib_path)
            .ok_or_else(|| format!("Library '{}' loaded nahi hai. Pehle load karo.", lib_path))?;
        let c_name = CString::new(fn_name)
            .map_err(|e| format!("Galat function name: {}", e))?;
        let ptr = unsafe { dlsym(handle, c_name.as_ptr()) };
        if ptr.is_null() {
            let err = unsafe { dlerror() };
            let msg = if err.is_null() {
                "symbol not found".to_string()
            } else {
                unsafe { std::ffi::CStr::from_ptr(err).to_string_lossy().to_string() }
            };
            return Err(format!("Symbol '{}' nahi mila: {}", fn_name, msg));
        }
        self.symbols.insert(key, ptr);
        Ok(ptr)
    }

    /// Look up a symbol via alias.
    pub fn lookup_symbol_by_alias(&mut self, alias: &str, fn_name: &str) -> Result<*mut c_void, String> {
        let lib_path = self.resolve_alias(alias)
            .ok_or_else(|| format!("Library alias '{}' register nahi hai.", alias))?
            .to_string();
        self.lookup_symbol(&lib_path, fn_name)
    }

    /// Call a C function pointer with up to 8 i64 arguments.
    /// Returns the i64 return value.
    pub unsafe fn call_fn_ptr(fn_ptr: *mut c_void, args: &[i64]) -> i64 {
        if fn_ptr.is_null() {
            return 0;
        }
        type CFn = unsafe extern "C" fn(i64, i64, i64, i64, i64, i64, i64, i64) -> i64;
        let f: CFn = std::mem::transmute(fn_ptr);
        let a0 = args.get(0).copied().unwrap_or(0);
        let a1 = args.get(1).copied().unwrap_or(0);
        let a2 = args.get(2).copied().unwrap_or(0);
        let a3 = args.get(3).copied().unwrap_or(0);
        let a4 = args.get(4).copied().unwrap_or(0);
        let a5 = args.get(5).copied().unwrap_or(0);
        let a6 = args.get(6).copied().unwrap_or(0);
        let a7 = args.get(7).copied().unwrap_or(0);
        f(a0, a1, a2, a3, a4, a5, a6, a7)
    }

    /// Convenience: load library + lookup symbol + call in one step.
    pub fn call_c_function(
        &mut self,
        lib_path: &str,
        fn_name: &str,
        args: &[i64],
    ) -> Result<i64, String> {
        self.load_library(lib_path)?;
        let ptr = self.lookup_symbol(lib_path, fn_name)?;
        let result = unsafe { FfiRegistry::call_fn_ptr(ptr, args) };
        Ok(result)
    }

    /// Summary of loaded libraries and symbols.
    pub fn summary(&self) {
        if self.handles.is_empty() && self.aliases.is_empty() {
            println!("  (koi FFI library loaded nahi hai)");
            return;
        }
        for (alias, path) in &self.aliases {
            println!("  ├── {} → {}", alias, path);
        }
        for (key, _) in &self.symbols {
            println!("  │   └── {}", key);
        }
        println!("  ╰ Total: {} library(ies), {} symbol(s)",
            self.handles.len(), self.symbols.len());
    }
}

impl Default for FfiRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffi_registry_new() {
        let reg = FfiRegistry::new();
        assert!(reg.handles.is_empty());
        assert!(reg.symbols.is_empty());
        assert!(reg.aliases.is_empty());
    }

    #[test]
    fn test_alias_registration() {
        let mut reg = FfiRegistry::new();
        reg.register_alias("mylib", "/usr/lib/libtest.so");
        assert_eq!(reg.resolve_alias("mylib"), Some("/usr/lib/libtest.so"));
        assert_eq!(reg.resolve_alias("nonexistent"), None);
    }

    #[test]
    fn test_load_nonexistent_library() {
        let mut reg = FfiRegistry::new();
        let result = reg.load_library("/nonexistent/lib.so");
        assert!(result.is_err());
    }
}
