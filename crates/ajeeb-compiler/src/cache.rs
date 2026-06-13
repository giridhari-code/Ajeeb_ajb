use crate::ast::{BinOp, Expr, Pattern, Stmt, TypeAnnot};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Clone, Debug)]
pub struct CacheEntry {
    pub source_path: PathBuf,
    pub source_mtime: SystemTime,
    pub stmts: Vec<Stmt>,
}

pub struct ModuleCache {
    cache_dir: PathBuf,
    // For tracking source file timestamps
    source_times: Vec<(PathBuf, SystemTime)>,
}

impl ModuleCache {
    pub fn new(cache_dir: PathBuf) -> Self {
        ModuleCache {
            cache_dir,
            source_times: Vec::new(),
        }
    }

    pub fn add_source(&mut self, path: &Path) {
        if let Ok(mtime) = fs::metadata(path).and_then(|m| m.modified()) {
            self.source_times.push((path.to_path_buf(), mtime));
        }
    }

    // Check if cache is valid: all cached mtimes match current file mtimes
    fn validate(&self, hash: u64) -> bool {
        let bin_path = self.cache_dir.join(format!("{:016x}.bin", hash));
        if !bin_path.exists() {
            return false;
        }
        // Read stored mtimes from the .bin file (first section contains mtime data)
        let data = match fs::read(&bin_path) {
            Ok(d) => d,
            Err(_) => return false,
        };
        let mut cursor = std::io::Cursor::new(data.as_slice());
        let stored_count = match read_u64_le(&mut cursor) {
            Some(n) => n as usize,
            None => return false,
        };
        for _ in 0..stored_count {
            let path_len = match read_u64_le(&mut cursor) {
                Some(n) => n as usize,
                None => return false,
            };
            let mut path_bytes = vec![0u8; path_len];
            if cursor.read(&mut path_bytes).ok() != Some(path_len) {
                return false;
            }
            let stored_path = String::from_utf8_lossy(&path_bytes).to_string();
            let stored_mtime_secs = match read_u64_le(&mut cursor) {
                Some(n) => n,
                None => return false,
            };
            let stored_mtime_nanos = match read_u64_le(&mut cursor) {
                Some(n) => n,
                None => return false,
            };
            // Check current mtime
            if let Ok(meta) = fs::metadata(&stored_path) {
                if let Ok(mtime) = meta.modified() {
                    let dur = mtime.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
                    if dur.as_secs() != stored_mtime_secs || dur.subsec_nanos() as u64 != stored_mtime_nanos {
                        return false;
                    }
                } else {
                    return false;
                }
            } else {
                return false;
            }
        }
        true
    }

    // Hash based only on the entry (first) source path so the filename is stable across both
    // load and save. Mtime validation for ALL sources is done inside the .bin file.
    fn compute_hash(&self) -> u64 {
        let mut h: u64 = 0xdeadbeefcafe1234;
        if let Some((path, _)) = self.source_times.first() {
            let p = path.to_string_lossy();
            for b in p.bytes() {
                h = h.wrapping_mul(16777619) ^ b as u64;
            }
        }
        h
    }

    pub fn load(&self) -> Option<Vec<Stmt>> {
        let hash = self.compute_hash();
        if !self.validate(hash) {
            return None;
        }
        let cache_path = self.cache_dir.join(format!("{:016x}.bin", hash));
        let data = fs::read(&cache_path).ok()?;
        let mut cursor = std::io::Cursor::new(data.as_slice());

        // Skip mtime block (we already validated)
        let count = read_u64_le(&mut cursor)? as usize;
        for _ in 0..count {
            let path_len = read_u64_le(&mut cursor)? as usize;
            cursor.set_position(cursor.position() + path_len as u64);
            cursor.set_position(cursor.position() + 16); // skip two u64 mtime fields
        }

        // Read statements
        let stmt_count = read_u64_le(&mut cursor)? as usize;
        let mut stmts = Vec::with_capacity(stmt_count);
        for _ in 0..stmt_count {
            stmts.push(read_stmt(&mut cursor)?);
        }
        Some(stmts)
    }

    pub fn save(&self, stmts: &[Stmt]) {
        fs::create_dir_all(&self.cache_dir).ok();
        let hash = self.compute_hash();

        let mut data = Vec::new();

        // Write mtime block for validation
        write_u64_le(&mut data, self.source_times.len() as u64);
        for (path, mtime) in &self.source_times {
            let p = path.to_string_lossy();
            write_u64_le(&mut data, p.len() as u64);
            data.extend_from_slice(p.as_bytes());
            let dur = mtime.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
            write_u64_le(&mut data, dur.as_secs());
            write_u64_le(&mut data, dur.subsec_nanos() as u64);
        }

        // Write statements
        write_u64_le(&mut data, stmts.len() as u64);
        for stmt in stmts {
            write_stmt(&mut data, stmt);
        }

        // Write to .bin file (contains both mtime block and serialized statements)
        let bin_path = self.cache_dir.join(format!("{:016x}.bin", hash));
        fs::write(&bin_path, &data).ok();
    }
}

// ── Binary Serialization Helpers ─────────────────────────────────

fn write_u64_le(data: &mut Vec<u8>, v: u64) {
    data.extend_from_slice(&v.to_le_bytes());
}

fn read_u64_le(cursor: &mut std::io::Cursor<&[u8]>) -> Option<u64> {
    let mut buf = [0u8; 8];
    if cursor.read(&mut buf).ok()? != 8 {
        return None;
    }
    Some(u64::from_le_bytes(buf))
}

fn write_type_annot(data: &mut Vec<u8>, t: &TypeAnnot) {
    match t {
        TypeAnnot::Int => write_u64_le(data, 0),
        TypeAnnot::Float => write_u64_le(data, 1),
        TypeAnnot::String => write_u64_le(data, 2),
        TypeAnnot::Bool => write_u64_le(data, 3),
        TypeAnnot::Void => write_u64_le(data, 4),
        TypeAnnot::Array(inner) => {
            write_u64_le(data, 5);
            write_type_annot(data, inner);
        }
        TypeAnnot::Class(name) => {
            write_u64_le(data, 6);
            write_string(data, name);
        }
        TypeAnnot::Generic(name) => {
            write_u64_le(data, 7);
            write_string(data, name);
        }
        TypeAnnot::Parameterized { base, args } => {
            write_u64_le(data, 8);
            write_type_annot(data, base);
            write_u64_le(data, args.len() as u64);
            for a in args {
                write_type_annot(data, a);
            }
        }
    }
}

fn read_type_annot(cursor: &mut std::io::Cursor<&[u8]>) -> Option<TypeAnnot> {
    match read_u64_le(cursor)? {
        0 => Some(TypeAnnot::Int),
        1 => Some(TypeAnnot::Float),
        2 => Some(TypeAnnot::String),
        3 => Some(TypeAnnot::Bool),
        4 => Some(TypeAnnot::Void),
        5 => {
            let inner = read_type_annot(cursor)?;
            Some(TypeAnnot::Array(Box::new(inner)))
        }
        6 => {
            let name = read_string(cursor)?;
            Some(TypeAnnot::Class(name))
        }
        7 => {
            let name = read_string(cursor)?;
            Some(TypeAnnot::Generic(name))
        }
        8 => {
            let base = read_type_annot(cursor)?;
            let count = read_u64_le(cursor)? as usize;
            let mut args = Vec::with_capacity(count);
            for _ in 0..count {
                args.push(read_type_annot(cursor)?);
            }
            Some(TypeAnnot::Parameterized { base: Box::new(base), args })
        }
        _ => None,
    }
}

fn write_string(data: &mut Vec<u8>, s: &str) {
    write_u64_le(data, s.len() as u64);
    data.extend_from_slice(s.as_bytes());
}

fn read_string(cursor: &mut std::io::Cursor<&[u8]>) -> Option<String> {
    let len = read_u64_le(cursor)? as usize;
    let mut buf = vec![0u8; len];
    if cursor.read(&mut buf).ok()? != len {
        return None;
    }
    Some(String::from_utf8_lossy(&buf).to_string())
}

fn write_expr(data: &mut Vec<u8>, expr: &Expr) {
    match expr {
        Expr::Number(n, ..) => {
            write_u64_le(data, 0);
            write_u64_le(data, *n as u64);
        }
        Expr::FloatLit(f, ..) => {
            write_u64_le(data, 1);
            write_u64_le(data, f.to_bits());
        }
        Expr::StringLit(s, ..) => {
            write_u64_le(data, 2);
            write_string(data, s);
        }
        Expr::Bool(b, ..) => {
            write_u64_le(data, 3);
            write_u64_le(data, if *b { 1 } else { 0 });
        }
        Expr::Ident(name, ..) => {
            write_u64_le(data, 4);
            write_string(data, name);
        }
        Expr::Binary { left, op, right, .. } => {
            write_u64_le(data, 5);
            write_expr(data, left);
            write_u64_le(data, match op {
                BinOp::Add => 0, BinOp::Sub => 1, BinOp::Mul => 2,
                BinOp::Div => 3, BinOp::Eq => 4, BinOp::Neq => 5,
                BinOp::Lt => 6, BinOp::Gt => 7, BinOp::Le => 8,
                BinOp::Ge => 9, BinOp::And => 10, BinOp::Or => 11,
            });
            write_expr(data, right);
        }
        Expr::Assign { name, value, .. } => {
            write_u64_le(data, 6);
            write_string(data, name);
            write_expr(data, value);
        }
        Expr::IndexAssign { obj, index, value, .. } => {
            write_u64_le(data, 7);
            write_expr(data, obj);
            write_expr(data, index);
            write_expr(data, value);
        }
        Expr::FnCall { name, args, .. } => {
            write_u64_le(data, 8);
            write_string(data, name);
            write_u64_le(data, args.len() as u64);
            for a in args {
                write_expr(data, a);
            }
        }
        Expr::GenericCall { name, type_args, args, .. } => {
            write_u64_le(data, 9);
            write_string(data, name);
            write_u64_le(data, type_args.len() as u64);
            for t in type_args {
                write_type_annot(data, t);
            }
            write_u64_le(data, args.len() as u64);
            for a in args {
                write_expr(data, a);
            }
        }
        Expr::MethodCall { obj, method, args, .. } => {
            write_u64_le(data, 10);
            write_expr(data, obj);
            write_string(data, method);
            write_u64_le(data, args.len() as u64);
            for a in args {
                write_expr(data, a);
            }
        }
        Expr::New { class_name, .. } => {
            write_u64_le(data, 11);
            write_string(data, class_name);
        }
        Expr::ArrayLit(items, ..) => {
            write_u64_le(data, 12);
            write_u64_le(data, items.len() as u64);
            for item in items {
                write_expr(data, item);
            }
        }
        Expr::Index { obj, index, .. } => {
            write_u64_le(data, 13);
            write_expr(data, obj);
            write_expr(data, index);
        }
        Expr::Field { obj, field, .. } => {
            write_u64_le(data, 14);
            write_expr(data, obj);
            write_string(data, field);
        }
        Expr::FieldAssign { obj, field, value, .. } => {
            write_u64_le(data, 15);
            write_expr(data, obj);
            write_string(data, field);
            write_expr(data, value);
        }
        Expr::UnaryMinus(val, ..) => {
            write_u64_le(data, 16);
            write_expr(data, val);
        }
        Expr::UnaryNot(val, ..) => {
            write_u64_le(data, 17);
            write_expr(data, val);
        }
        Expr::Group(val, ..) => {
            write_u64_le(data, 18);
            write_expr(data, val);
        }
        Expr::StructLit { struct_name, fields, .. } => {
            write_u64_le(data, 19);
            write_string(data, struct_name);
            write_u64_le(data, fields.len() as u64);
            for (name, val) in fields {
                write_string(data, name);
                write_expr(data, val);
            }
        }
        Expr::EnumRef { enum_name, variant, .. } => {
            write_u64_le(data, 20);
            write_string(data, enum_name);
            write_string(data, variant);
        }
        Expr::EnumCtor { enum_name, variant, args, .. } => {
            write_u64_le(data, 21);
            write_string(data, enum_name);
            write_string(data, variant);
            write_u64_le(data, args.len() as u64);
            for a in args {
                write_expr(data, a);
            }
        }
        Expr::Match { value, arms, .. } => {
            write_u64_le(data, 22);
            write_expr(data, value);
            write_u64_le(data, arms.len() as u64);
            for arm in arms {
                write_pattern(data, &arm.pattern);
                write_expr(data, &arm.body);
                if let Some(block) = &arm.body_block {
                    write_u64_le(data, 1);
                    write_u64_le(data, block.len() as u64);
                    for s in block {
                        write_stmt(data, s);
                    }
                } else {
                    write_u64_le(data, 0);
                }
            }
        }
    }
}

fn read_expr(cursor: &mut std::io::Cursor<&[u8]>) -> Option<Expr> {
    Some(match read_u64_le(cursor)? {
        0 => Expr::Number(read_u64_le(cursor)? as i64, 0, 0),
        1 => Expr::FloatLit(f64::from_bits(read_u64_le(cursor)?), 0, 0),
        2 => Expr::StringLit(read_string(cursor)?, 0, 0),
        3 => Expr::Bool(read_u64_le(cursor)? != 0, 0, 0),
        4 => Expr::Ident(read_string(cursor)?, 0, 0),
        5 => {
            let left = read_expr(cursor)?;
            let op = match read_u64_le(cursor)? {
                0 => BinOp::Add, 1 => BinOp::Sub, 2 => BinOp::Mul,
                3 => BinOp::Div, 4 => BinOp::Eq, 5 => BinOp::Neq,
                6 => BinOp::Lt, 7 => BinOp::Gt, 8 => BinOp::Le,
                9 => BinOp::Ge, 10 => BinOp::And, 11 => BinOp::Or,
                _ => return None,
            };
            let right = read_expr(cursor)?;
            Expr::Binary { left: Box::new(left), op, right: Box::new(right), line: 0, col: 0 }
        }
        6 => {
            let name = read_string(cursor)?;
            let value = read_expr(cursor)?;
            Expr::Assign { name, value: Box::new(value), line: 0, col: 0 }
        }
        7 => {
            let obj = read_expr(cursor)?;
            let index = read_expr(cursor)?;
            let value = read_expr(cursor)?;
            Expr::IndexAssign { obj: Box::new(obj), index: Box::new(index), value: Box::new(value), line: 0, col: 0 }
        }
        8 => {
            let name = read_string(cursor)?;
            let arg_count = read_u64_le(cursor)? as usize;
            let mut args = Vec::with_capacity(arg_count);
            for _ in 0..arg_count {
                args.push(read_expr(cursor)?);
            }
            Expr::FnCall { name, args, line: 0, col: 0 }
        }
        9 => {
            let name = read_string(cursor)?;
            let ta_count = read_u64_le(cursor)? as usize;
            let mut type_args = Vec::with_capacity(ta_count);
            for _ in 0..ta_count {
                type_args.push(read_type_annot(cursor)?);
            }
            let arg_count = read_u64_le(cursor)? as usize;
            let mut args = Vec::with_capacity(arg_count);
            for _ in 0..arg_count {
                args.push(read_expr(cursor)?);
            }
            Expr::GenericCall { name, type_args, args, line: 0, col: 0 }
        }
        10 => {
            let obj = read_expr(cursor)?;
            let method = read_string(cursor)?;
            let arg_count = read_u64_le(cursor)? as usize;
            let mut args = Vec::with_capacity(arg_count);
            for _ in 0..arg_count {
                args.push(read_expr(cursor)?);
            }
            Expr::MethodCall { obj: Box::new(obj), method, args, line: 0, col: 0 }
        }
        11 => Expr::New { class_name: read_string(cursor)?, line: 0, col: 0 },
        12 => {
            let count = read_u64_le(cursor)? as usize;
            let mut items = Vec::with_capacity(count);
            for _ in 0..count {
                items.push(read_expr(cursor)?);
            }
            Expr::ArrayLit(items, 0, 0)
        }
        13 => {
            let obj = read_expr(cursor)?;
            let index = read_expr(cursor)?;
            Expr::Index { obj: Box::new(obj), index: Box::new(index), line: 0, col: 0 }
        }
        14 => {
            let obj = read_expr(cursor)?;
            let field = read_string(cursor)?;
            Expr::Field { obj: Box::new(obj), field, line: 0, col: 0 }
        }
        15 => {
            let obj = read_expr(cursor)?;
            let field = read_string(cursor)?;
            let value = read_expr(cursor)?;
            Expr::FieldAssign { obj: Box::new(obj), field, value: Box::new(value), line: 0, col: 0 }
        }
        16 => Expr::UnaryMinus(Box::new(read_expr(cursor)?), 0, 0),
        17 => Expr::UnaryNot(Box::new(read_expr(cursor)?), 0, 0),
        18 => Expr::Group(Box::new(read_expr(cursor)?), 0, 0),
        19 => {
            let struct_name = read_string(cursor)?;
            let field_count = read_u64_le(cursor)? as usize;
            let mut fields = Vec::with_capacity(field_count);
            for _ in 0..field_count {
                let name = read_string(cursor)?;
                let val = read_expr(cursor)?;
                fields.push((name, val));
            }
            Expr::StructLit { struct_name, fields, line: 0, col: 0 }
        }
        20 => Expr::EnumRef { enum_name: read_string(cursor)?, variant: read_string(cursor)?, line: 0, col: 0 },
        21 => {
            let enum_name = read_string(cursor)?;
            let variant = read_string(cursor)?;
            let arg_count = read_u64_le(cursor)? as usize;
            let mut args = Vec::with_capacity(arg_count);
            for _ in 0..arg_count {
                args.push(read_expr(cursor)?);
            }
            Expr::EnumCtor { enum_name, variant, args, line: 0, col: 0 }
        }
        22 => {
            let value = read_expr(cursor)?;
            let arm_count = read_u64_le(cursor)? as usize;
            let mut arms = Vec::with_capacity(arm_count);
            for _ in 0..arm_count {
                let pattern = read_pattern(cursor)?;
                let body = read_expr(cursor)?;
                let has_block = read_u64_le(cursor)? != 0;
                let body_block = if has_block {
                    let count = read_u64_le(cursor)? as usize;
                    let mut block = Vec::with_capacity(count);
                    for _ in 0..count {
                        block.push(read_stmt(cursor)?);
                    }
                    Some(block)
                } else {
                    None
                };
                arms.push(crate::ast::MatchArm { pattern, body, body_block });
            }
            Expr::Match { value: Box::new(value), arms, line: 0, col: 0 }
        }
        _ => return None,
    })
}

fn write_pattern(data: &mut Vec<u8>, pat: &Pattern) {
    match pat {
        Pattern::Wildcard => write_u64_le(data, 0),
        Pattern::EnumVariant { enum_name, variant, bindings } => {
            write_u64_le(data, 1);
            write_string(data, enum_name);
            write_string(data, variant);
            write_u64_le(data, bindings.len() as u64);
            for b in bindings {
                write_string(data, b);
            }
        }
        Pattern::Int(n) => {
            write_u64_le(data, 2);
            write_u64_le(data, *n as u64);
        }
        Pattern::String(s) => {
            write_u64_le(data, 3);
            write_string(data, s);
        }
    }
}

fn read_pattern(cursor: &mut std::io::Cursor<&[u8]>) -> Option<Pattern> {
    Some(match read_u64_le(cursor)? {
        0 => Pattern::Wildcard,
        1 => {
            let enum_name = read_string(cursor)?;
            let variant = read_string(cursor)?;
            let count = read_u64_le(cursor)? as usize;
            let mut bindings = Vec::with_capacity(count);
            for _ in 0..count {
                bindings.push(read_string(cursor)?);
            }
            Pattern::EnumVariant { enum_name, variant, bindings }
        }
        2 => Pattern::Int(read_u64_le(cursor)? as i64),
        3 => Pattern::String(read_string(cursor)?),
        _ => return None,
    })
}

pub fn write_stmt(data: &mut Vec<u8>, stmt: &Stmt) {
    match stmt {
        Stmt::Let { name, type_ann, value, pub_, .. } => {
            write_u64_le(data, 0);
            write_string(data, name);
            if let Some(ta) = type_ann {
                write_u64_le(data, 1);
                write_type_annot(data, ta);
            } else {
                write_u64_le(data, 0);
            }
            write_expr(data, value);
            write_u64_le(data, if *pub_ { 1 } else { 0 });
        }
        Stmt::Const { name, type_ann, value, pub_, .. } => {
            write_u64_le(data, 1);
            write_string(data, name);
            if let Some(ta) = type_ann {
                write_u64_le(data, 1);
                write_type_annot(data, ta);
            } else {
                write_u64_le(data, 0);
            }
            write_expr(data, value);
            write_u64_le(data, if *pub_ { 1 } else { 0 });
        }
        Stmt::If { condition, then_block, else_block, .. } => {
            write_u64_le(data, 2);
            write_expr(data, condition);
            write_u64_le(data, then_block.len() as u64);
            for s in then_block {
                write_stmt(data, s);
            }
            if let Some(eb) = else_block {
                write_u64_le(data, 1);
                write_u64_le(data, eb.len() as u64);
                for s in eb {
                    write_stmt(data, s);
                }
            } else {
                write_u64_le(data, 0);
            }
        }
        Stmt::While { condition, body, .. } => {
            write_u64_le(data, 3);
            write_expr(data, condition);
            write_u64_le(data, body.len() as u64);
            for s in body {
                write_stmt(data, s);
            }
        }
        Stmt::ForLoop { init, condition, update, body, .. } => {
            write_u64_le(data, 4);
            write_stmt(data, init);
            write_expr(data, condition);
            write_stmt(data, update);
            write_u64_le(data, body.len() as u64);
            for s in body {
                write_stmt(data, s);
            }
        }
        Stmt::Break { .. } => write_u64_le(data, 5),
        Stmt::Continue { .. } => write_u64_le(data, 6),
        Stmt::Return { value, .. } => {
            write_u64_le(data, 7);
            if let Some(v) = value {
                write_u64_le(data, 1);
                write_expr(data, v);
            } else {
                write_u64_le(data, 0);
            }
        }
        Stmt::Expr(expr, ..) => {
            write_u64_le(data, 8);
            write_expr(data, expr);
        }
        Stmt::FnDef { name, type_params, params, return_type, body, pub_, .. } => {
            write_u64_le(data, 9);
            write_string(data, name);
            write_u64_le(data, type_params.len() as u64);
            for tp in type_params {
                write_string(data, tp);
            }
            write_u64_le(data, params.len() as u64);
            for (pname, ptype) in params {
                write_string(data, pname);
                write_type_annot(data, ptype);
            }
            write_type_annot(data, return_type);
            write_u64_le(data, if *pub_ { 1 } else { 0 });
            write_u64_le(data, body.len() as u64);
            for s in body {
                write_stmt(data, s);
            }
        }
        Stmt::Class { name, fields, methods, pub_, .. } => {
            write_u64_le(data, 10);
            write_string(data, name);
            write_u64_le(data, fields.len() as u64);
            for f in fields {
                write_string(data, &f.name);
                write_type_annot(data, &f.type_ann);
                write_u64_le(data, if f.pub_ { 1 } else { 0 });
            }
            write_u64_le(data, methods.len() as u64);
            for m in methods {
                write_stmt(data, m);
            }
            write_u64_le(data, if *pub_ { 1 } else { 0 });
        }
        Stmt::Import(import) => {
            write_u64_le(data, 11);
            write_u64_le(data, import.path.len() as u64);
            for p in &import.path {
                write_string(data, p);
            }
            if let Some(alias) = &import.alias {
                write_u64_le(data, 1);
                write_string(data, alias);
            } else {
                write_u64_le(data, 0);
            }
        }
        Stmt::StructDef { name, type_params, fields, pub_, .. } => {
            write_u64_le(data, 12);
            write_string(data, name);
            write_u64_le(data, type_params.len() as u64);
            for tp in type_params {
                write_string(data, tp);
            }
            write_u64_le(data, fields.len() as u64);
            for f in fields {
                write_string(data, &f.name);
                write_type_annot(data, &f.type_ann);
            }
            write_u64_le(data, if *pub_ { 1 } else { 0 });
        }
        Stmt::EnumDef { name, type_params, variants, pub_, .. } => {
            write_u64_le(data, 13);
            write_string(data, name);
            write_u64_le(data, type_params.len() as u64);
            for tp in type_params {
                write_string(data, tp);
            }
            write_u64_le(data, variants.len() as u64);
            for v in variants {
                write_string(data, &v.name);
                write_u64_le(data, v.fields.len() as u64);
                for ft in &v.fields {
                    write_type_annot(data, ft);
                }
            }
            write_u64_le(data, if *pub_ { 1 } else { 0 });
        }
        Stmt::TraitDef { name, methods, pub_, .. } => {
            write_u64_le(data, 14);
            write_string(data, name);
            write_u64_le(data, methods.len() as u64);
            for m in methods {
                write_string(data, &m.name);
                write_u64_le(data, m.params.len() as u64);
                for (pname, ptype) in &m.params {
                    write_string(data, pname);
                    write_type_annot(data, ptype);
                }
                write_type_annot(data, &m.return_type);
            }
            write_u64_le(data, if *pub_ { 1 } else { 0 });
        }
        Stmt::ImplBlock { trait_name, type_name, methods, .. } => {
            write_u64_le(data, 15);
            write_string(data, trait_name);
            write_string(data, type_name);
            write_u64_le(data, methods.len() as u64);
            for m in methods {
                write_stmt(data, m);
            }
        }
    }
}

fn read_stmt(cursor: &mut std::io::Cursor<&[u8]>) -> Option<Stmt> {
    Some(match read_u64_le(cursor)? {
        0 => {
            let name = read_string(cursor)?;
            let has_type = read_u64_le(cursor)? != 0;
            let type_ann = if has_type { Some(read_type_annot(cursor)?) } else { None };
            let value = read_expr(cursor)?;
            let pub_ = read_u64_le(cursor)? != 0;
            Stmt::Let { name, type_ann, value, pub_, line: 0, col: 0 }
        }
        1 => {
            let name = read_string(cursor)?;
            let has_type = read_u64_le(cursor)? != 0;
            let type_ann = if has_type { Some(read_type_annot(cursor)?) } else { None };
            let value = read_expr(cursor)?;
            let pub_ = read_u64_le(cursor)? != 0;
            Stmt::Const { name, type_ann, value, pub_, line: 0, col: 0 }
        }
        2 => {
            let condition = read_expr(cursor)?;
            let then_len = read_u64_le(cursor)? as usize;
            let mut then_block = Vec::with_capacity(then_len);
            for _ in 0..then_len { then_block.push(read_stmt(cursor)?); }
            let has_else = read_u64_le(cursor)? != 0;
            let else_block = if has_else {
                let else_len = read_u64_le(cursor)? as usize;
                let mut eb = Vec::with_capacity(else_len);
                for _ in 0..else_len { eb.push(read_stmt(cursor)?); }
                Some(eb)
            } else { None };
            Stmt::If { condition, then_block, else_block, line: 0, col: 0 }
        }
        3 => {
            let condition = read_expr(cursor)?;
            let body_len = read_u64_le(cursor)? as usize;
            let mut body = Vec::with_capacity(body_len);
            for _ in 0..body_len { body.push(read_stmt(cursor)?); }
            Stmt::While { condition, body, line: 0, col: 0 }
        }
        4 => {
            let init = read_stmt(cursor)?;
            let condition = read_expr(cursor)?;
            let update = read_stmt(cursor)?;
            let body_len = read_u64_le(cursor)? as usize;
            let mut body = Vec::with_capacity(body_len);
            for _ in 0..body_len { body.push(read_stmt(cursor)?); }
            Stmt::ForLoop { init: Box::new(init), condition, update: Box::new(update), body, line: 0, col: 0 }
        }
        5 => Stmt::Break { line: 0, col: 0 },
        6 => Stmt::Continue { line: 0, col: 0 },
        7 => {
            let has_value = read_u64_le(cursor)? != 0;
            let value = if has_value { Some(read_expr(cursor)?) } else { None };
            Stmt::Return { value, line: 0, col: 0 }
        }
        8 => {
            let expr = read_expr(cursor)?;
            Stmt::Expr(expr, 0, 0)
        }
        9 => {
            let name = read_string(cursor)?;
            let tp_count = read_u64_le(cursor)? as usize;
            let mut type_params = Vec::with_capacity(tp_count);
            for _ in 0..tp_count { type_params.push(read_string(cursor)?); }
            let param_count = read_u64_le(cursor)? as usize;
            let mut params = Vec::with_capacity(param_count);
            for _ in 0..param_count {
                let pname = read_string(cursor)?;
                let ptype = read_type_annot(cursor)?;
                params.push((pname, ptype));
            }
            let return_type = read_type_annot(cursor)?;
            let pub_ = read_u64_le(cursor)? != 0;
            let body_len = read_u64_le(cursor)? as usize;
            let mut body = Vec::with_capacity(body_len);
            for _ in 0..body_len { body.push(read_stmt(cursor)?); }
            Stmt::FnDef { name, type_params, params, return_type, body, pub_, line: 0, col: 0 }
        }
        10 => {
            let name = read_string(cursor)?;
            let field_count = read_u64_le(cursor)? as usize;
            let mut fields = Vec::with_capacity(field_count);
            for _ in 0..field_count {
                let fname = read_string(cursor)?;
                let ftype = read_type_annot(cursor)?;
                let fpub = read_u64_le(cursor)? != 0;
                fields.push(crate::ast::ClassField { name: fname, type_ann: ftype, pub_: fpub });
            }
            let method_count = read_u64_le(cursor)? as usize;
            let mut methods = Vec::with_capacity(method_count);
            for _ in 0..method_count { methods.push(read_stmt(cursor)?); }
            let pub_ = read_u64_le(cursor)? != 0;
            Stmt::Class { name, fields, methods, pub_, line: 0, col: 0 }
        }
        11 => {
            let path_len = read_u64_le(cursor)? as usize;
            let mut path = Vec::with_capacity(path_len);
            for _ in 0..path_len { path.push(read_string(cursor)?); }
            let has_alias = read_u64_le(cursor)? != 0;
            let alias = if has_alias { Some(read_string(cursor)?) } else { None };
            Stmt::Import(crate::ast::ImportDecl { path, alias, line: 0, col: 0 })
        }
        12 => {
            let name = read_string(cursor)?;
            let tp_count = read_u64_le(cursor)? as usize;
            let mut type_params = Vec::with_capacity(tp_count);
            for _ in 0..tp_count { type_params.push(read_string(cursor)?); }
            let field_count = read_u64_le(cursor)? as usize;
            let mut fields = Vec::with_capacity(field_count);
            for _ in 0..field_count {
                let fname = read_string(cursor)?;
                let ftype = read_type_annot(cursor)?;
                fields.push(crate::ast::StructField { name: fname, type_ann: ftype });
            }
            let pub_ = read_u64_le(cursor)? != 0;
            Stmt::StructDef { name, type_params, fields, pub_, line: 0, col: 0 }
        }
        13 => {
            let name = read_string(cursor)?;
            let tp_count = read_u64_le(cursor)? as usize;
            let mut type_params = Vec::with_capacity(tp_count);
            for _ in 0..tp_count { type_params.push(read_string(cursor)?); }
            let variant_count = read_u64_le(cursor)? as usize;
            let mut variants = Vec::with_capacity(variant_count);
            for _ in 0..variant_count {
                let vname = read_string(cursor)?;
                let field_count = read_u64_le(cursor)? as usize;
                let mut fields = Vec::with_capacity(field_count);
                for _ in 0..field_count { fields.push(read_type_annot(cursor)?); }
                variants.push(crate::ast::EnumVariantDef { name: vname, fields });
            }
            let pub_ = read_u64_le(cursor)? != 0;
            Stmt::EnumDef { name, type_params, variants, pub_, line: 0, col: 0 }
        }
        14 => {
            let name = read_string(cursor)?;
            let method_count = read_u64_le(cursor)? as usize;
            let mut methods = Vec::with_capacity(method_count);
            for _ in 0..method_count {
                let mname = read_string(cursor)?;
                let param_count = read_u64_le(cursor)? as usize;
                let mut params = Vec::with_capacity(param_count);
                for _ in 0..param_count {
                    let pname = read_string(cursor)?;
                    let ptype = read_type_annot(cursor)?;
                    params.push((pname, ptype));
                }
                let return_type = read_type_annot(cursor)?;
                methods.push(crate::ast::TraitMethod { name: mname, params, return_type });
            }
            let pub_ = read_u64_le(cursor)? != 0;
            Stmt::TraitDef { name, methods, pub_, line: 0, col: 0 }
        }
        15 => {
            let trait_name = read_string(cursor)?;
            let type_name = read_string(cursor)?;
            let method_count = read_u64_le(cursor)? as usize;
            let mut methods = Vec::with_capacity(method_count);
            for _ in 0..method_count { methods.push(read_stmt(cursor)?); }
            Stmt::ImplBlock { trait_name, type_name, methods, line: 0, col: 0 }
        }
        _ => return None,
    })
}
