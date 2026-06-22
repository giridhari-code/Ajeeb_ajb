use super::{Evaluator, RuntimeValue, FrameInfo, print_value};
use crate::ast::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::CString;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::os::raw::{c_char, c_void};
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

extern "C" {
    fn dlopen(filename: *const c_char, flags: i32) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlclose(handle: *mut c_void) -> i32;
    fn dlerror() -> *mut c_char;
}

impl Evaluator {
    pub(super) fn exec_fn_call_body(&mut self, name: &str, arg_vals: &[RuntimeValue]) -> RuntimeValue {
        match name {
            "print" | "println" => {
                let nl = name == "println";
                for a in arg_vals {
                    match a {
                        RuntimeValue::Int(n) => print!("{}", n),
                        RuntimeValue::Float(f) => print!("{}", f),
                        RuntimeValue::String(s) => {
                            let s_clean: String = s.borrow().chars().take_while(|&c| c != '\0').collect();
                            print!("{}", s_clean);
                        }
                        RuntimeValue::Bool(b) => print!("{}", b),
                        RuntimeValue::Array(arr_rc) => {
                            let arr = arr_rc.borrow();
                            print!("[");
                            for (i, e) in arr.iter().enumerate() {
                                if i > 0 {
                                    print!(", ");
                                }
                                match e {
                                    RuntimeValue::Int(n) => print!("{}", n),
                                    RuntimeValue::String(s) => {
                                        let s_clean: String = s.borrow().chars().take_while(|&c| c != '\0').collect();
                                        print!("\"{}\"", s_clean);
                                    }
                                    RuntimeValue::Bool(b) => print!("{}", b),
                                    RuntimeValue::Array(inner_rc) => {
                                        let inner = inner_rc.borrow();
                                        print!("[");
                                        for (j, ee) in inner.iter().enumerate() {
                                            if j > 0 {
                                                print!(", ");
                                            }
                                            match ee {
                                                RuntimeValue::Int(n) => print!("{}", n),
                                                RuntimeValue::String(s) => {
                                                    print!("\"{}\"", s.borrow())
                                                }
                                                RuntimeValue::Bool(b) => print!("{}", b),
                                                _ => print!("<?>"),
                                            }
                                        }
                                        print!("]");
                                    }
                                    _ => print!("<?>"),
                                }
                            }
                            print!("]");
                        }
                        RuntimeValue::ClassInstance { class_name, .. } => {
                            print!("<{} instance>", class_name)
                        }
                        RuntimeValue::StructInstance { name: sn, .. } => {
                            print!("<{} struct>", sn)
                        }
                        RuntimeValue::EnumVariant { enum_name, variant, data } => {
                            print!("{}::{}", enum_name, variant);
                            if !data.is_empty() {
                                print!("(");
                                for (i, d) in data.iter().enumerate() {
                                    if i > 0 { print!(", "); }
                                    print_value(d);
                                }
                                print!(")");
                            }
                        }
                        RuntimeValue::Void => print!("void"),
                        RuntimeValue::Return(v) => print!("<return {:?}>", v),
                        RuntimeValue::Break => print!("break"),
                        RuntimeValue::Continue => print!("continue"),
                    }
                }
                if nl {
                    println!();
                }
                RuntimeValue::Void
            }
            "itoa" => {
                if let Some(RuntimeValue::Int(n)) = arg_vals.first() {
                    RuntimeValue::String(Rc::new(RefCell::new(n.to_string())))
                } else {
                    RuntimeValue::String(Rc::new(RefCell::new("0".to_string())))
                }
            }
            "len" => {
                if let Some(RuntimeValue::String(s)) = arg_vals.first() {
                    RuntimeValue::Int(s.borrow().len() as i64)
                } else if let Some(RuntimeValue::Array(arr)) = arg_vals.first() {
                    RuntimeValue::Int(arr.borrow().len() as i64)
                } else {
                    RuntimeValue::Int(0)
                }
            }
            "arr_len" => {
                if let Some(RuntimeValue::Array(arr)) = arg_vals.first() {
                    RuntimeValue::Int(arr.borrow().len() as i64)
                } else {
                    RuntimeValue::Int(0)
                }
            }
            "charCode" => {
                if arg_vals.len() >= 2 {
                    let s = match &arg_vals[0] {
                        RuntimeValue::String(s) => Some(s.clone()),
                        RuntimeValue::Int(ptr) => self.int_to_string.get(ptr).cloned(),
                        _ => None,
                    };
                    if let (Some(s), RuntimeValue::Int(i)) = (s, &arg_vals[1]) {
                        let idx = *i as usize;
                        let b = s.borrow();
                        if idx < b.len() {
                            let val = b.as_bytes()[idx] as i64;
                            return RuntimeValue::Int(val);
                        }
                    }
                }
                RuntimeValue::Int(0)
            }
            "strcmp" => {
                if arg_vals.len() >= 2 {
                    if let (RuntimeValue::String(a), RuntimeValue::String(b)) =
                        (&arg_vals[0], &arg_vals[1])
                    {
                        let av: &str = &a.borrow();
                        let bv: &str = &b.borrow();
                        return RuntimeValue::Int(if av < bv {
                            -1
                        } else if av > bv {
                            1
                        } else {
                            0
                        });
                    }
                }
                RuntimeValue::Int(0)
            }
            "readFile" => {
                let path = if let Some(RuntimeValue::String(s)) = arg_vals.first() {
                    s.borrow().clone()
                } else {
                    String::new()
                };
                let content = std::fs::read_to_string(&path).unwrap_or_default();
                RuntimeValue::String(Rc::new(RefCell::new(content)))
            }
            "writeFile" => {
                if arg_vals.len() >= 2 {
                    if let (RuntimeValue::String(path), RuntimeValue::String(content)) =
                        (&arg_vals[0], &arg_vals[1])
                    {
                        let bytes: Vec<u8> = content.borrow().bytes().take_while(|&b| b != 0).collect();
                        let _ = std::fs::write(path.borrow().as_str(), &bytes);
                    }
                }
                RuntimeValue::Void
            }
            "writeAppend" => {
                if arg_vals.len() >= 2 {
                    if let (RuntimeValue::String(path), RuntimeValue::String(content)) =
                        (&arg_vals[0], &arg_vals[1])
                    {
                        let path_str = path.borrow().clone();
                        let f = self.open_files.entry(path_str).or_insert_with_key(|key| {
                            OpenOptions::new()
                                .create(true)
                                .append(true)
                                .open(key.as_str())
                                .expect("writeAppend: failed to open file")
                        });
                        let bytes: Vec<u8> = content.borrow().bytes().take_while(|&b| b != 0).collect();
                        let _ = f.write_all(&bytes);
                    }
                }
                RuntimeValue::Void
            }
            "exec" => {
                if let Some(RuntimeValue::String(cmd)) = arg_vals.first() {
                    let cmd_clean: String = cmd.borrow().chars().take_while(|&c| c != '\0').collect();
                    let exit_code = std::process::Command::new("sh")
                        .args(["-c", &cmd_clean])
                        .status()
                        .map(|s| s.code().unwrap_or(-1))
                        .unwrap_or(-1);
                    return RuntimeValue::Int(exit_code as i64);
                }
                RuntimeValue::Int(0)
            }
            "mkdir" => {
                if let Some(RuntimeValue::String(path)) = arg_vals.first() {
                    let path_clean: String = path.borrow().chars().take_while(|&c| c != '\0').collect();
                    let exit_code = std::process::Command::new("mkdir")
                        .args(["-p", &path_clean])
                        .status()
                        .map(|s| s.code().unwrap_or(-1))
                        .unwrap_or(-1);
                    return RuntimeValue::Int(exit_code as i64);
                }
                RuntimeValue::Int(0)
            }
            "readArg" => {
                let idx = if let Some(RuntimeValue::Int(n)) = arg_vals.first() {
                    *n as usize
                } else {
                    0
                };
                if idx < self.program_args.len() {
                    RuntimeValue::String(Rc::new(RefCell::new(self.program_args[idx].clone())))
                } else {
                    RuntimeValue::String(Rc::new(RefCell::new(String::new())))
                }
            }
            "getStateBuf" => {
                let key = "__state__".to_string();
                self.int_buffers
                    .entry(key.clone())
                    .or_insert_with(|| vec![0i64; 16384]);
                RuntimeValue::String(Rc::new(RefCell::new(key)))
            }
            "getOutbuf" => {
                self.outbuf_string.borrow_mut().clear();
                RuntimeValue::String(self.outbuf_string.clone())
            }
            "rdB" | "getInt" => {
                if arg_vals.len() >= 2 {
                    if let (RuntimeValue::String(buf_name), RuntimeValue::Int(offset)) =
                        (&arg_vals[0], &arg_vals[1])
                    {
                        let idx = (offset / 8) as usize;
                        if let Some(buf) = self.int_buffers.get(buf_name.borrow().as_str()) {
                            if idx < buf.len() {
                                return RuntimeValue::Int(buf[idx]);
                            }
                        }
                    }
                }
                RuntimeValue::Int(0)
            }
            "wrB" | "setInt" => {
                if arg_vals.len() >= 3 {
                    if let RuntimeValue::String(buf_name) = &arg_vals[0] {
                        if let RuntimeValue::Int(offset) = &arg_vals[1] {
                            let idx = (offset / 8) as usize;
                            let name = buf_name.borrow().clone();
                            let buf = self
                                .int_buffers
                                .entry(name)
                                .or_insert_with(|| vec![0i64; 16384]);
                            if idx < buf.len() {
                                match &arg_vals[2] {
                                    RuntimeValue::Int(v) => buf[idx] = *v,
                                    RuntimeValue::String(s) => {
                                        let ptr = self.next_string_ptr;
                                        self.next_string_ptr += 1;
                                        self.int_to_string.insert(ptr, s.clone());
                                        buf[idx] = ptr;
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                RuntimeValue::Void
            }
            "strcpy" => {
                if arg_vals.len() >= 2 {
                    if let (RuntimeValue::String(dst), RuntimeValue::String(src)) =
                        (&arg_vals[0], &arg_vals[1])
                    {
                        *dst.borrow_mut() = src.borrow().clone();
                    }
                }
                RuntimeValue::Void
            }
            "strSet" => {
                if arg_vals.len() >= 3 {
                    if let (
                        RuntimeValue::String(s),
                        RuntimeValue::Int(idx),
                        RuntimeValue::Int(ch),
                    ) = (&arg_vals[0], &arg_vals[1], &arg_vals[2])
                    {
                        let i = *idx as usize;
                        let mut buf = s.borrow_mut();
                        let blen = buf.len();
                        if i >= blen {
                            buf.extend(std::iter::repeat_n('\0', i + 1 - blen));
                        }
                        buf.replace_range(
                            i..=i,
                            &String::from(char::from_u32(*ch as u32).unwrap_or('\0')),
                        );
                    }
                }
                RuntimeValue::Void
            }
            "writeByte" => {
                if arg_vals.len() >= 2 {
                    if let (RuntimeValue::String(path), RuntimeValue::Int(byte)) =
                        (&arg_vals[0], &arg_vals[1])
                    {
                        let path_str = path.borrow().clone();
                        let f = self.open_files.entry(path_str).or_insert_with_key(|key| {
                            OpenOptions::new()
                                .create(true)
                                .append(true)
                                .open(key.as_str())
                                .expect("writeByte: failed to open file")
                        });
                        let _ = f.write_all(&[*byte as u8]);
                    }
                }
                RuntimeValue::Void
            }
            "chr" => {
                if arg_vals.len() >= 2 {
                    if let (RuntimeValue::String(s), RuntimeValue::Int(i)) =
                        (&arg_vals[0], &arg_vals[1])
                    {
                        let idx = *i as usize;
                        let b = s.borrow();
                        if idx < b.len() {
                            let c = b.as_bytes()[idx] as char;
                            return RuntimeValue::String(Rc::new(RefCell::new(c.to_string())));
                        }
                    }
                }
                RuntimeValue::String(Rc::new(RefCell::new("".to_string())))
            }
            "chr_str" => {
                if let Some(RuntimeValue::Int(code)) = arg_vals.first() {
                    let c = (*code).max(0).min(255) as u8 as char;
                    RuntimeValue::String(Rc::new(RefCell::new(c.to_string())))
                } else {
                    RuntimeValue::Int(0)
                }
            }
            "getStr" => {
                if let Some(RuntimeValue::Int(ptr)) = arg_vals.first() {
                    if let Some(s) = self.int_to_string.get(ptr) {
                        return RuntimeValue::String(s.clone());
                    }
                }
                RuntimeValue::String(Rc::new(RefCell::new(String::new())))
            }
            "rdPos" => {
                if let Some(RuntimeValue::String(buf_name)) = arg_vals.first() {
                    if let Some(buf) = self.int_buffers.get(buf_name.borrow().as_str()) {
                        if !buf.is_empty() {
                            return RuntimeValue::Int(buf[0]);
                        }
                    }
                }
                RuntimeValue::Int(0)
            }
            "wrPos" => {
                if arg_vals.len() >= 2 {
                    if let (RuntimeValue::String(buf_name), RuntimeValue::Int(v)) =
                        (&arg_vals[0], &arg_vals[1])
                    {
                        let name = buf_name.borrow().clone();
                        let buf = self
                            .int_buffers
                            .entry(name)
                            .or_insert_with(|| vec![0i64; 16384]);
                        if !buf.is_empty() {
                            buf[0] = *v;
                        }
                    }
                }
                RuntimeValue::Void
            }
            "isDigit" => {
                if let Some(RuntimeValue::Int(c)) = arg_vals.first() {
                    return RuntimeValue::Int((*c >= 48 && *c <= 57) as i64);
                }
                RuntimeValue::Int(0)
            }
            "isAlpha" => {
                if let Some(RuntimeValue::Int(c)) = arg_vals.first() {
                    let uc = *c as u8;
                    return RuntimeValue::Int(
                        ((65..=90).contains(&uc) || (97..=122).contains(&uc) || uc == 95) as i64,
                    );
                }
                RuntimeValue::Int(0)
            }
            "isAlphaNum" => {
                if let Some(RuntimeValue::Int(c)) = arg_vals.first() {
                    let uc = *c as u8;
                    return RuntimeValue::Int(
                        ((48..=57).contains(&uc)
                            || (65..=90).contains(&uc)
                            || (97..=122).contains(&uc)
                            || uc == 95) as i64,
                    );
                }
                RuntimeValue::Int(0)
            }
            "isSpace" => {
                if let Some(RuntimeValue::Int(c)) = arg_vals.first() {
                    return RuntimeValue::Int((*c == 32 || *c == 9 || *c == 10 || *c == 13) as i64);
                }
                RuntimeValue::Int(0)
            }
            "strcmp_ajeeb" => {
                if arg_vals.len() >= 2 {
                    if let (RuntimeValue::String(a), RuntimeValue::String(b)) =
                        (&arg_vals[0], &arg_vals[1])
                    {
                        let a_trim: String =
                            a.borrow().chars().take_while(|&c| c != '\0').collect();
                        let b_trim: String =
                            b.borrow().chars().take_while(|&c| c != '\0').collect();
                        if a_trim == b_trim {
                            return RuntimeValue::Int(0);
                        }
                        if a_trim < b_trim {
                            return RuntimeValue::Int(-1);
                        }
                        return RuntimeValue::Int(1);
                    }
                }
                RuntimeValue::Int(0)
            }
            "str_concat" => {
                if arg_vals.len() >= 2 {
                    if let (RuntimeValue::String(a), RuntimeValue::String(b)) =
                        (&arg_vals[0], &arg_vals[1])
                    {
                        let result = format!("{}{}", a.borrow(), b.borrow());
                        return RuntimeValue::String(Rc::new(RefCell::new(result)));
                    }
                }
                RuntimeValue::String(Rc::new(RefCell::new(String::new())))
            }
            "substring" => {
                let s_guard: std::cell::Ref<String>;
                let s: &str = match arg_vals.first() {
                    Some(RuntimeValue::String(ss)) => { s_guard = ss.borrow(); &*s_guard }
                    _ => "",
                };
                let start = arg_vals
                    .get(1)
                    .and_then(|a| {
                        if let RuntimeValue::Int(i) = a {
                            Some(*i as usize)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(0);
                let end = arg_vals
                    .get(2)
                    .and_then(|a| {
                        if let RuntimeValue::Int(i) = a {
                            Some(*i as usize)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(s.len());
                let end = end.min(s.len());
                let sub: String = s
                    .chars()
                    .skip(start)
                    .take(end.saturating_sub(start))
                    .collect();
                RuntimeValue::String(Rc::new(RefCell::new(sub)))
            }
            "indexOf" => {
                let sg1: std::cell::Ref<String>;
                let sg2: std::cell::Ref<String>;
                let s: &str = match arg_vals.first() {
                    Some(RuntimeValue::String(ss)) => { sg1 = ss.borrow(); &*sg1 }
                    _ => "",
                };
                let search: &str = match arg_vals.get(1) {
                    Some(RuntimeValue::String(ss)) => { sg2 = ss.borrow(); &*sg2 }
                    _ => "",
                };
                let start: usize = match arg_vals.get(2) {
                    Some(RuntimeValue::Int(n)) => { (*n).max(0) as usize }
                    _ => 0,
                };
                let haystack = if start < s.len() { &s[start..] } else { "" };
                if let Some(pos) = haystack.find(search) {
                    RuntimeValue::Int((start + pos) as i64)
                } else {
                    RuntimeValue::Int(-1)
                }
            }
            "contains" => {
                let sg1: std::cell::Ref<String>;
                let sg2: std::cell::Ref<String>;
                let s: &str = match arg_vals.first() {
                    Some(RuntimeValue::String(ss)) => { sg1 = ss.borrow(); &*sg1 }
                    _ => "",
                };
                let search: &str = match arg_vals.get(1) {
                    Some(RuntimeValue::String(ss)) => { sg2 = ss.borrow(); &*sg2 }
                    _ => "",
                };
                RuntimeValue::Int(if s.contains(search) { 1 } else { 0 })
            }
            "toUpperCase" => {
                let s_guard: std::cell::Ref<String>;
                let s: &str = match arg_vals.first() {
                    Some(RuntimeValue::String(ss)) => { s_guard = ss.borrow(); &*s_guard }
                    _ => "",
                };
                RuntimeValue::String(Rc::new(RefCell::new(s.to_uppercase())))
            }
            "toLowerCase" => {
                let s_guard: std::cell::Ref<String>;
                let s: &str = match arg_vals.first() {
                    Some(RuntimeValue::String(ss)) => { s_guard = ss.borrow(); &*s_guard }
                    _ => "",
                };
                RuntimeValue::String(Rc::new(RefCell::new(s.to_lowercase())))
            }
            "trim" => {
                let s_guard: std::cell::Ref<String>;
                let s: &str = match arg_vals.first() {
                    Some(RuntimeValue::String(ss)) => { s_guard = ss.borrow(); &*s_guard }
                    _ => "",
                };
                RuntimeValue::String(Rc::new(RefCell::new(s.trim().to_string())))
            }
            "split" => {
                let sg1: std::cell::Ref<String>;
                let sg2: std::cell::Ref<String>;
                let s: &str = match arg_vals.first() {
                    Some(RuntimeValue::String(ss)) => { sg1 = ss.borrow(); &*sg1 }
                    _ => "",
                };
                let delim: &str = match arg_vals.get(1) {
                    Some(RuntimeValue::String(ss)) => { sg2 = ss.borrow(); &*sg2 }
                    _ => "",
                };
                let parts: Vec<RuntimeValue> = if delim.is_empty() {
                    s.chars()
                        .map(|c| RuntimeValue::String(Rc::new(RefCell::new(c.to_string()))))
                        .collect()
                } else {
                    s.split(delim)
                        .map(|p| RuntimeValue::String(Rc::new(RefCell::new(p.to_string()))))
                        .collect()
                };
                RuntimeValue::Array(Rc::new(RefCell::new(parts)))
            }
            "replace" => {
                let sg1: std::cell::Ref<String>;
                let sg2: std::cell::Ref<String>;
                let sg3: std::cell::Ref<String>;
                let s: &str = match arg_vals.first() {
                    Some(RuntimeValue::String(ss)) => { sg1 = ss.borrow(); &*sg1 }
                    _ => "",
                };
                let from: &str = match arg_vals.get(1) {
                    Some(RuntimeValue::String(ss)) => { sg2 = ss.borrow(); &*sg2 }
                    _ => "",
                };
                let to: &str = match arg_vals.get(2) {
                    Some(RuntimeValue::String(ss)) => { sg3 = ss.borrow(); &*sg3 }
                    _ => "",
                };
                RuntimeValue::String(Rc::new(RefCell::new(s.replace(from, to))))
            }
            "startsWith" => {
                let sg1: std::cell::Ref<String>;
                let sg2: std::cell::Ref<String>;
                let s: &str = match arg_vals.first() {
                    Some(RuntimeValue::String(ss)) => { sg1 = ss.borrow(); &*sg1 }
                    _ => "",
                };
                let prefix: &str = match arg_vals.get(1) {
                    Some(RuntimeValue::String(ss)) => { sg2 = ss.borrow(); &*sg2 }
                    _ => "",
                };
                RuntimeValue::Int(if s.starts_with(prefix) { 1 } else { 0 })
            }
            "endsWith" => {
                let sg1: std::cell::Ref<String>;
                let sg2: std::cell::Ref<String>;
                let s: &str = match arg_vals.first() {
                    Some(RuntimeValue::String(ss)) => { sg1 = ss.borrow(); &*sg1 }
                    _ => "",
                };
                let suffix: &str = match arg_vals.get(1) {
                    Some(RuntimeValue::String(ss)) => { sg2 = ss.borrow(); &*sg2 }
                    _ => "",
                };
                RuntimeValue::Int(if s.ends_with(suffix) { 1 } else { 0 })
            }
            "tcp_listen" => {
                if let Some(RuntimeValue::Int(port)) = arg_vals.first() {
                    match TcpListener::bind(format!("127.0.0.1:{}", port)) {
                        Ok(listener) => {
                            let fd = self.next_handle;
                            self.next_handle += 1;
                            self.tcp_listeners.insert(fd, listener);
                            RuntimeValue::Int(fd)
                        }
                        Err(e) => {
                            eprintln!("[TCP] listen error on port {}: {}", port, e);
                            RuntimeValue::Int(0)
                        }
                    }
                } else {
                    RuntimeValue::Int(0)
                }
            }
            "tcp_accept" => {
                if let Some(RuntimeValue::Int(fd)) = arg_vals.first() {
                    if let Some(listener) = self.tcp_listeners.get(fd) {
                        match listener.accept() {
                            Ok((stream, _addr)) => {
                                let client_fd = self.next_handle;
                                self.next_handle += 1;
                                stream.set_nonblocking(true).ok();
                                self.tcp_clients.insert(client_fd, stream);
                                RuntimeValue::Int(client_fd)
                            }
                            Err(_) => RuntimeValue::Int(0),
                        }
                    } else {
                        RuntimeValue::Int(0)
                    }
                } else {
                    RuntimeValue::Int(0)
                }
            }
            "tcp_read" => {
                let fd = arg_vals.first().and_then(|a| if let RuntimeValue::Int(f) = a { Some(*f) } else { None }).unwrap_or(0);
                let max = arg_vals.get(1).and_then(|a| if let RuntimeValue::Int(m) = a { Some(*m as usize) } else { None }).unwrap_or(4096);
                if let Some(stream) = self.tcp_clients.get_mut(&fd) {
                    let mut buf = vec![0u8; max];
                    match stream.read(&mut buf) {
                        Ok(n) if n > 0 => {
                            buf.truncate(n);
                            RuntimeValue::String(Rc::new(RefCell::new(
                                String::from_utf8_lossy(&buf).to_string()
                            )))
                        }
                        _ => RuntimeValue::String(Rc::new(RefCell::new(String::new()))),
                    }
                } else {
                    RuntimeValue::String(Rc::new(RefCell::new(String::new())))
                }
            }
            "tcp_write" => {
                let fd = arg_vals.first().and_then(|a| if let RuntimeValue::Int(f) = a { Some(*f) } else { None }).unwrap_or(0);
                let data_guard: std::cell::Ref<String>;
                let data: &[u8] = match arg_vals.get(1) {
                    Some(RuntimeValue::String(s)) => { data_guard = s.borrow(); data_guard.as_bytes() }
                    _ => &[],
                };
                if let Some(stream) = self.tcp_clients.get_mut(&fd) {
                    let _ = stream.write_all(data);
                }
                RuntimeValue::Void
            }
            "tcp_close" => {
                let fd = arg_vals.first().and_then(|a| if let RuntimeValue::Int(f) = a { Some(*f) } else { None }).unwrap_or(0);
                self.tcp_clients.remove(&fd);
                self.tcp_listeners.remove(&fd);
                RuntimeValue::Void
            }
            "tcp_connect" => {
                let host = match arg_vals.first() {
                    Some(RuntimeValue::String(s)) => s.borrow().clone(),
                    _ => return RuntimeValue::Int(0),
                };
                let port = match arg_vals.get(1) {
                    Some(RuntimeValue::Int(p)) => *p,
                    _ => return RuntimeValue::Int(0),
                };
                match TcpStream::connect(format!("{}:{}", host, port)) {
                    Ok(stream) => {
                        stream.set_nonblocking(true).ok();
                        let fd = self.next_handle;
                        self.next_handle += 1;
                        self.tcp_clients.insert(fd, stream);
                        RuntimeValue::Int(fd)
                    }
                    Err(e) => {
                        eprintln!("[TCP] connect error to {}:{}: {}", host, port, e);
                        RuntimeValue::Int(0)
                    }
                }
            }
            "dns_lookup" => {
                let hostname = match arg_vals.first() {
                    Some(RuntimeValue::String(s)) => s.borrow().clone(),
                    _ => return RuntimeValue::String(Rc::new(RefCell::new(String::new()))),
                };
                match format!("{}:0", hostname).to_socket_addrs() {
                    Ok(mut addrs) => {
                        if let Some(addr) = addrs.next() {
                            RuntimeValue::String(Rc::new(RefCell::new(addr.ip().to_string())))
                        } else {
                            RuntimeValue::String(Rc::new(RefCell::new(String::new())))
                        }
                    }
                    Err(e) => {
                        eprintln!("[DNS] lookup error for '{}': {}", hostname, e);
                        RuntimeValue::String(Rc::new(RefCell::new(String::new())))
                    }
                }
            }
            "tls_connect" => {
                eprintln!("[TLS] tls_connect fallback to plain TCP (use native mode for real TLS)");
                let host = match arg_vals.first() {
                    Some(RuntimeValue::String(s)) => s.borrow().clone(),
                    _ => return RuntimeValue::Int(0),
                };
                let port = match arg_vals.get(1) {
                    Some(RuntimeValue::Int(p)) => *p,
                    _ => return RuntimeValue::Int(0),
                };
                match TcpStream::connect(format!("{}:{}", host, port)) {
                    Ok(stream) => {
                        stream.set_nonblocking(true).ok();
                        let fd = self.next_handle;
                        self.next_handle += 1;
                        self.tcp_clients.insert(fd, stream);
                        RuntimeValue::Int(fd)
                    }
                    Err(e) => {
                        eprintln!("[TLS] connect error to {}:{}: {}", host, port, e);
                        RuntimeValue::Int(0)
                    }
                }
            }
            "tls_read" => {
                let fd = arg_vals.first().and_then(|a| if let RuntimeValue::Int(f) = a { Some(*f) } else { None }).unwrap_or(0);
                let max = arg_vals.get(1).and_then(|a| if let RuntimeValue::Int(m) = a { Some(*m as usize) } else { None }).unwrap_or(4096);
                if let Some(stream) = self.tcp_clients.get_mut(&fd) {
                    let mut buf = vec![0u8; max];
                    match stream.read(&mut buf) {
                        Ok(n) if n > 0 => {
                            buf.truncate(n);
                            RuntimeValue::String(Rc::new(RefCell::new(
                                String::from_utf8_lossy(&buf).to_string()
                            )))
                        }
                        _ => RuntimeValue::String(Rc::new(RefCell::new(String::new()))),
                    }
                } else {
                    RuntimeValue::String(Rc::new(RefCell::new(String::new())))
                }
            }
            "tls_write" => {
                let fd = arg_vals.first().and_then(|a| if let RuntimeValue::Int(f) = a { Some(*f) } else { None }).unwrap_or(0);
                let data_guard: std::cell::Ref<String>;
                let data: &[u8] = match arg_vals.get(1) {
                    Some(RuntimeValue::String(s)) => { data_guard = s.borrow(); data_guard.as_bytes() }
                    _ => &[],
                };
                if let Some(stream) = self.tcp_clients.get_mut(&fd) {
                    let _ = stream.write_all(data);
                }
                RuntimeValue::Void
            }
            "tls_close" => {
                let fd = arg_vals.first().and_then(|a| if let RuntimeValue::Int(f) = a { Some(*f) } else { None }).unwrap_or(0);
                self.tcp_clients.remove(&fd);
                self.tcp_listeners.remove(&fd);
                RuntimeValue::Void
            }
            "now_ms" => {
                let ms = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as i64;
                RuntimeValue::Int(ms)
            }
            "sqlite_open" => {
                let path = arg_vals.first().and_then(|a| if let RuntimeValue::String(s) = a { Some(s.borrow().clone()) } else { None }).unwrap_or_default();
                let handle = self.next_handle;
                self.next_handle += 1;
                self.sqlite_dbs.insert(handle, path);
                RuntimeValue::Int(handle)
            }
            "sqlite_close" => {
                let handle = arg_vals.first().and_then(|a| if let RuntimeValue::Int(h) = a { Some(*h) } else { None }).unwrap_or(0);
                self.sqlite_dbs.remove(&handle);
                RuntimeValue::Void
            }
            "sqlite_exec" => {
                RuntimeValue::Int(0)
            }
            "sqlite_query" => {
                RuntimeValue::Array(Rc::new(RefCell::new(Vec::new())))
            }
            "sqlite_last_error" => {
                RuntimeValue::String(Rc::new(RefCell::new("SQLite available only in native mode".to_string())))
            }
            "assert_eq" => {
                if arg_vals.len() >= 2 {
                    let got = format!("{:?}", arg_vals[0]);
                    let expected = format!("{:?}", arg_vals[1]);
                    if got != expected {
                        eprintln!("FAIL: assert_eq expected '{}' got '{}'", expected, got);
                    }
                }
                RuntimeValue::Void
            }
            "assert_neq" => {
                if arg_vals.len() >= 2 {
                    let got = format!("{:?}", arg_vals[0]);
                    let expected = format!("{:?}", arg_vals[1]);
                    if got == expected {
                        eprintln!("FAIL: assert_neq expected different values, got '{}'", got);
                    }
                }
                RuntimeValue::Void
            }
            "assert_contains" => {
                if let (Some(RuntimeValue::String(s)), Some(RuntimeValue::String(sub))) = (arg_vals.first(), arg_vals.get(1)) {
                    if !s.borrow().contains(&sub.borrow().as_str()) {
                        eprintln!("FAIL: '{}' does not contain '{}'", s.borrow(), sub.borrow());
                    }
                }
                RuntimeValue::Void
            }
            "lib_open" => {
                let path = match arg_vals.first() {
                    Some(RuntimeValue::String(s)) => s.borrow().clone(),
                    _ => String::new(),
                };
                if path.is_empty() {
                    return RuntimeValue::Int(-1);
                }
                match CString::new(path.as_str()) {
                    Ok(cpath) => unsafe {
                        let handle = dlopen(cpath.as_ptr(), 1 | 0x001);
                        if handle.is_null() {
                            let err = dlerror();
                            if !err.is_null() {
                                let msg = std::ffi::CStr::from_ptr(err).to_string_lossy().to_string();
                                eprintln!("[FFI] dlopen error: {}", msg);
                            }
                            RuntimeValue::Int(-1)
                        } else {
                            RuntimeValue::Int(handle as i64)
                        }
                    },
                    Err(_) => RuntimeValue::Int(-1),
                }
            }
            "lib_sym" => {
                let handle = match arg_vals.first() {
                    Some(RuntimeValue::Int(h)) => *h,
                    _ => return RuntimeValue::Int(0),
                };
                let sym_name = match arg_vals.get(1) {
                    Some(RuntimeValue::String(s)) => s.borrow().clone(),
                    _ => return RuntimeValue::Int(0),
                };
                match CString::new(sym_name.as_str()) {
                    Ok(cname) => unsafe {
                        let ptr = dlsym(handle as *mut c_void, cname.as_ptr());
                        RuntimeValue::Int(ptr as i64)
                    },
                    Err(_) => RuntimeValue::Int(0),
                }
            }
            "lib_call" => {
                let fn_ptr = match arg_vals.first() {
                    Some(RuntimeValue::Int(p)) => *p,
                    _ => return RuntimeValue::Int(0),
                };
                let args_arr = match arg_vals.get(1) {
                    Some(RuntimeValue::Array(a)) => a.borrow().clone(),
                    _ => Vec::new(),
                };
                let _ret_type = match arg_vals.get(2) {
                    Some(RuntimeValue::Int(r)) => *r,
                    _ => 0,
                };
                let c_args: Vec<i64> = args_arr.iter().map(|v| match v {
                    RuntimeValue::Int(n) => *n,
                    _ => 0,
                }).collect();
                unsafe {
                    let result = match c_args.len() {
                        0 => {
                            let f: unsafe extern "C" fn() -> i64 = std::mem::transmute(fn_ptr as usize);
                            f()
                        }
                        1 => {
                            let f: unsafe extern "C" fn(i64) -> i64 = std::mem::transmute(fn_ptr as usize);
                            f(c_args[0])
                        }
                        2 => {
                            let f: unsafe extern "C" fn(i64, i64) -> i64 = std::mem::transmute(fn_ptr as usize);
                            f(c_args[0], c_args[1])
                        }
                        3 => {
                            let f: unsafe extern "C" fn(i64, i64, i64) -> i64 = std::mem::transmute(fn_ptr as usize);
                            f(c_args[0], c_args[1], c_args[2])
                        }
                        4 => {
                            let f: unsafe extern "C" fn(i64, i64, i64, i64) -> i64 = std::mem::transmute(fn_ptr as usize);
                            f(c_args[0], c_args[1], c_args[2], c_args[3])
                        }
                        5 => {
                            let f: unsafe extern "C" fn(i64, i64, i64, i64, i64) -> i64 = std::mem::transmute(fn_ptr as usize);
                            f(c_args[0], c_args[1], c_args[2], c_args[3], c_args[4])
                        }
                        6 => {
                            let f: unsafe extern "C" fn(i64, i64, i64, i64, i64, i64) -> i64 = std::mem::transmute(fn_ptr as usize);
                            f(c_args[0], c_args[1], c_args[2], c_args[3], c_args[4], c_args[5])
                        }
                        _ => 0,
                    };
                    RuntimeValue::Int(result)
                }
            }
            "call_fn" => {
                if let Some(RuntimeValue::String(fn_name)) = arg_vals.first() {
                    let name: &str = &fn_name.borrow();
                    return self.exec_fn_call_body(name, &arg_vals[1..]);
                }
                RuntimeValue::Array(Rc::new(RefCell::new(Vec::new())))
            }
            _ => {
                if self.class_fields.contains_key(name) && arg_vals.is_empty() {
                    let mut fields = HashMap::new();
                    if let Some(field_list) = self.class_fields.get(name) {
                        for f in field_list {
                            fields.insert(f.name.clone(), RuntimeValue::Int(0));
                        }
                    }
                    return RuntimeValue::ClassInstance {
                        class_name: name.to_string(),
                        fields,
                    };
                }
                if let Some((params, body, _)) = self.functions.get(name).cloned() {
                    self.push_scope();
                    for (i, (pname, _)) in params.iter().enumerate() {
                        let val = if i < arg_vals.len() {
                            arg_vals[i].clone()
                        } else {
                            RuntimeValue::Int(0)
                        };
                        self.insert_var(pname.clone(), val);
                    }
                    let mut result = RuntimeValue::Void;
                    for s in &body {
                        let r = self.exec_stmt(s);
                        if let RuntimeValue::Return(val) = r {
                            result = *val;
                            break;
                        }
                    }
                    self.pop_scope();
                    return result;
                } else {
                    self.print_stack_trace();
                    eprintln!(
                        "[ERROR] Unknown function '{}' called with {} args",
                        name,
                        arg_vals.len()
                    );
                }
                RuntimeValue::Void
            }
        }
    }
}
