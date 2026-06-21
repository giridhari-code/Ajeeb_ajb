use std::fs;
use std::path::Path;

pub fn extract_doc_comments(source: &str) -> Vec<(usize, String)> {
    let mut docs = Vec::new();
    for (i, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        if let Some(content) = trimmed.strip_prefix("///") {
            docs.push((i + 1, content.trim().to_string()));
        }
    }
    docs
}

pub fn generate_doc_html(name: &str, docs: &[(usize, String)]) -> String {
    let mut body = String::new();
    body.push_str(&format!("<h1>{}</h1>\n", html_escape(name)));
    body.push_str("<dl>\n");
    for (line, text) in docs {
        body.push_str(&format!("  <dt>Line {}</dt>\n  <dd>{}</dd>\n", line, html_escape(text)));
    }
    body.push_str("</dl>\n");

    format!(
        "<!DOCTYPE html>\n\
         <html lang=\"en\">\n\
         <head><meta charset=\"UTF-8\"><title>{}</title></head>\n\
         <body>\n\
         {}\n\
         </body>\n\
         </html>\n",
        html_escape(name),
        body
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn generate_project_docs(project_name: &str) -> Result<(), String> {
    let src_dir = Path::new("src");
    if !src_dir.exists() {
        return Err("src/ directory not found".to_string());
    }

    let docs_dir = Path::new("docs");
    fs::create_dir_all(docs_dir).map_err(|e| format!("Cannot create docs dir: {}", e))?;

    let mut entries: Vec<_> = fs::read_dir(src_dir)
        .map_err(|e| format!("Cannot read src/: {}", e))?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ex| ex == "ajb").unwrap_or(false))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in &entries {
        let path = entry.path();
        let source = fs::read_to_string(&path).map_err(|e| format!("Cannot read {:?}: {}", path, e))?;
        let docs = extract_doc_comments(&source);

        let stem = path.file_stem().unwrap().to_string_lossy().to_string();
        let html = if docs.is_empty() {
            format!(
                "<!DOCTYPE html>\n\
                 <html lang=\"en\">\n\
                 <head><meta charset=\"UTF-8\"><title>{} — {}</title></head>\n\
                 <body>\n\
                 <h1>{}</h1>\n\
                 <p>No documentation comments found.</p>\n\
                 </body>\n\
                 </html>\n",
                html_escape(project_name),
                html_escape(&stem),
                html_escape(&stem),
            )
        } else {
            generate_doc_html(&stem, &docs)
        };

        let out_path = docs_dir.join(format!("{}.html", stem));
        fs::write(&out_path, &html).map_err(|e| format!("Cannot write {:?}: {}", out_path, e))?;
        println!("📝 Generated {}", out_path.display());
    }

    let index_html = {
        let mut items = String::new();
        for entry in &entries {
            let stem = entry.path().file_stem().unwrap().to_string_lossy().to_string();
            items.push_str(&format!("  <li><a href=\"{}.html\">{}</a></li>\n", stem, stem));
        }
        format!(
            "<!DOCTYPE html>\n\
             <html lang=\"en\">\n\
             <head><meta charset=\"UTF-8\"><title>{} — Documentation</title></head>\n\
             <body>\n\
             <h1>{}</h1>\n\
             <ul>\n\
             {}\
             </ul>\n\
             </body>\n\
             </html>\n",
            html_escape(project_name),
            html_escape(project_name),
            items,
        )
    };
    fs::write(docs_dir.join("index.html"), &index_html).map_err(|e| format!("Cannot write index: {}", e))?;
    println!("📝 Generated {}", docs_dir.join("index.html").display());
    println!("✓ Documentation generated in docs/");

    Ok(())
}
