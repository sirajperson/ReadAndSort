use clap::{Arg, ArgAction, Command};
use std::path::{Path, PathBuf};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use std::ffi::OsStr;
use std::io::{self, BufRead};
use std::collections::HashMap;
use regex::Regex;
use chrono::{DateTime, Utc};
use colored::*;
use std::os::unix::fs::{FileTypeExt, PermissionsExt};

#[derive(Debug)]
enum SortField {
    Name,
    Date,
    Size,
    Type,
    Ext,
}

impl SortField {
    fn from_str(s: &str) -> Self {
        match s {
            "date" => SortField::Date,
            "size" => SortField::Size,
            "type" => SortField::Type,
            "ext" => SortField::Ext,
            _ => SortField::Name
        }
    }
}

#[derive(Debug)]
enum SortDirection {
    Asc,
    Desc,
}

impl SortDirection {
    fn from_str(s: &str) -> Self {
        match s {
            "desc" => SortDirection::Desc,
            _ => SortDirection::Asc
        }
    }
}

fn file_type_groups() -> HashMap<&'static str, Vec<&'static str>> {
    let mut groups = HashMap::new();
    groups.insert("web", vec!["html","htm","css","scss","less","js","jsx","ts","tsx"]);
    groups.insert("docs", vec!["md","txt","pdf","doc","docx","odt","rtf"]);
    groups.insert("images", vec!["jpg","jpeg","png","gif","svg","webp","bmp"]);
    groups.insert("code", vec!["py","java","cpp","c","h","hpp","cs","go","rs","php","rb","pl","scala","kt","swift"]);
    groups.insert("config", vec!["json","yaml","yml","toml","ini","conf","xml"]);
    groups.insert("data", vec!["csv","sql","db","sqlite"]);
    groups.insert("script", vec!["sh","bash","zsh","fish","ps1","bat","cmd"]);
    groups
}

#[derive(Debug)]
struct Config {
    max_depth: usize,
    project_dir: PathBuf,
    exclude_dirs: Vec<String>,
    output_format: String,
    show_content: bool,
    max_file_size: u64,
    file_types: Vec<String>,
    sort_by: SortField,
    sort_direction: SortDirection,
    sort_dirs_first: bool,
    content_filter: Option<Regex>,
    content_context: usize,
    whole_file: bool,
    highlight: bool,
    groups: HashMap<&'static str, Vec<&'static str>>,
}

fn main() {
    let matches = Command::new("rs")
        .version("1.0")
        .about("Maps and displays the source tree with syntax highlighting.")
        .long_about(
r#"Maps and displays the source tree with syntax highlighting, filtering, and sorting options.

Examples:
  rs ./src
      Show top-level entries in ./src

  rs -d 3 -t ext:py -c
      Show Python files 3 levels deep and include file contents

  rs --sort date -t group:code
      Show code files sorted by modification date

  rs -c -p "TODO" --highlight
      Show files containing "TODO" and highlight the matches

  rs -t ext:py -t group:web ./src
      Filter by more than one type (Python files OR files in the web group)
      Note: Append multiple '-t' flags for multiple filters instead of using a delimiter.
"#
        )
        .after_help(
r#"Type Filters:
  File Types:
    ext:EXTENSION   Show files with a specific extension (e.g., ext:py)
    group:GROUP     Show files from a specific group (e.g., group:web)

  Special Types:
    binary          Show binary files
    text            Show text files
    dir             Show directories
    socket          Show sockets
    pipe            Show pipes
    executable      Show executable files
    symlink         Show symbolic links
    device          Show device files
    hidden          Show hidden files
    empty           Show empty files
    archive         Show archive files
"#
        )
        .arg(
            Arg::new("directory")
                .help("Directory to start mapping from")
                .default_value(".")
                .num_args(1)
        )
        .arg(
            Arg::new("depth")
                .short('d')
                .long("depth")
                .help("Maximum directory depth (0 = unlimited)")
                .num_args(1)
                .default_value("1")
        )
        .arg(
            Arg::new("format")
                .short('f')
                .long("format")
                .help("Output format: markdown or text")
                .num_args(1)
                .default_value("markdown")
        )
        .arg(
            Arg::new("exclude")
                .short('e')
                .long("exclude")
                .help("Exclude directories or files by name (can be used multiple times)")
                .action(ArgAction::Append)
        )
        .arg(
            Arg::new("content")
                .short('c')
                .long("content")
                .help("Show file contents in the tree")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new("max_size")
                .short('s')
                .long("max-size")
                .help("Maximum file size in bytes for content display")
                .num_args(1)
                .default_value("100000")
        )
        .arg(
            Arg::new("type")
                .short('t')
                .long("type")
                .help("Filter results by type (can be used multiple times)")
                .action(ArgAction::Append)
        )
        .arg(
            Arg::new("pattern")
                .short('p')
                .long("pattern")
                .help("Show only content matching a specific pattern")
                .num_args(1)
        )
        .arg(
            Arg::new("context")
                .long("context")
                .help("Show N lines of context around matches")
                .num_args(1)
                .default_value("0")
        )
        .arg(
            Arg::new("whole_file")
                .long("whole-file")
                .help("Show the entire file if any line matches")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new("highlight")
                .long("highlight")
                .help("Highlight matching content")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new("sort")
                .long("sort")
                .help("Sort by: name,date,size,type,ext")
                .num_args(1)
                .default_value("name")
        )
        .arg(
            Arg::new("direction")
                .long("direction")
                .help("Sort direction: asc or desc")
                .num_args(1)
                .default_value("asc")
        )
        .arg(
            Arg::new("dirs_first")
                .long("dirs-first")
                .help("Show directories first")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new("no_dirs_first")
                .long("no-dirs-first")
                .help("Don't sort directories separately")
                .action(ArgAction::SetTrue)
        )
        .get_matches();

    let project_dir = PathBuf::from(matches.get_one::<String>("directory").unwrap());
    let max_depth = matches.get_one::<String>("depth")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    let output_format = matches.get_one::<String>("format").unwrap().to_string();
    let exclude_dirs: Vec<String> = matches.get_many::<String>("exclude")
        .map(|vals| vals.cloned().collect())
        .unwrap_or_default();
    let show_content = matches.get_flag("content");
    let max_file_size = matches.get_one::<String>("max_size")
        .and_then(|v| v.parse().ok())
        .unwrap_or(100000);
    let file_types: Vec<String> = matches.get_many::<String>("type")
        .map(|vals| vals.cloned().collect())
        .unwrap_or_default();
    let content_filter = matches.get_one::<String>("pattern")
        .map(|p| Regex::new(p).unwrap());
    let content_context = matches.get_one::<String>("context")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let whole_file = matches.get_flag("whole_file");
    let highlight = matches.get_flag("highlight");
    let sort_by = SortField::from_str(matches.get_one::<String>("sort").unwrap());
    let sort_direction = SortDirection::from_str(matches.get_one::<String>("direction").unwrap());
    let sort_dirs_first = if matches.get_flag("no_dirs_first") {
        false
    } else if matches.get_flag("dirs_first") {
        true
    } else {
        true
    };

    let groups = file_type_groups();

    let config = Config {
        max_depth,
        project_dir,
        exclude_dirs,
        output_format,
        show_content,
        max_file_size,
        file_types,
        sort_by,
        sort_direction,
        sort_dirs_first,
        content_filter,
        content_context,
        whole_file,
        highlight,
        groups,
    };

    if !config.project_dir.is_dir() {
        eprintln!("Error: '{}' is not a directory.", config.project_dir.display());
        std::process::exit(1);
    }

    // Print header
    if config.output_format == "markdown" {
        println!("# 📁 Project Source Tree: {}", config.project_dir.file_name().unwrap_or_else(|| OsStr::new(".")).to_string_lossy());
    } else {
        println!("Project Source Tree: {}", config.project_dir.file_name().unwrap_or_else(|| OsStr::new(".")).to_string_lossy());
    }
    println!("Generated on {}", Utc::now().to_rfc3339());
    if !config.file_types.is_empty() {
        println!("Filters: {:?}", config.file_types);
    }
    if let Some(ref pat) = config.content_filter {
        println!("Content Pattern: {}", pat);
    }
    if matches.get_one::<String>("sort").unwrap() != "name" || matches.get_one::<String>("direction").unwrap() != "asc" {
        println!("Sorting: {:?} ({:?})", config.sort_by, config.sort_direction);
    }
    println!();

    print_tree(&config.project_dir, "", &config, 1);

    println!();
    if config.output_format == "markdown" {
        println!("_End of source tree_");
    } else {
        println!("End of source tree");
    }
}

struct DirEntryExt {
    path: PathBuf,
    is_dir: bool,
    size: u64,
    modified: Option<SystemTime>,
    ext: Option<String>,
    filetype_desc: String,
}

fn print_tree(
    dir: &Path,
    prefix: &str,
    config: &Config,
    current_depth: usize,
) {
    if config.max_depth != 0 && current_depth > config.max_depth {
        return;
    }

    let mut entries = Vec::new();
    let read_dir = match fs::read_dir(dir) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{}Error reading directory '{}': {}", prefix, dir.display(), e);
            return;
        }
    };

    for entry_res in read_dir {
        if let Ok(entry) = entry_res {
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();

            if config.exclude_dirs.contains(&file_name_str.to_string()) {
                continue;
            }

            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            let is_dir = metadata.is_dir();
            if !matches_type_filter(&entry.path(), &config.file_types, is_dir, &config.groups) {
                continue;
            }

            let size = if !is_dir { metadata.len() } else { 0 };
            let modified = metadata.modified().ok();
            let ext = entry.path().extension().map(|e| e.to_string_lossy().to_string());
            let filetype_desc = file_type_description(&entry.path());

            entries.push(DirEntryExt {
                path: entry.path(),
                is_dir,
                size,
                modified,
                ext,
                filetype_desc,
            });
        }
    }

    sort_entries(&mut entries, &config.sort_by, &config.sort_direction, config.sort_dirs_first);

    for entry in entries {
        let name = entry.path.file_name().unwrap_or_else(|| OsStr::new("")).to_string_lossy();
        if entry.is_dir {
            let dir_info = if let SortField::Date = config.sort_by {
                if let Some(m) = entry.modified {
                    let dt: DateTime<Utc> = m.into();
                    format!(" (modified: {})", dt.to_rfc3339())
                } else {
                    "".to_string()
                }
            } else if let SortField::Size = config.sort_by {
                let count = fs::read_dir(&entry.path).map(|d| d.count()).unwrap_or(0);
                format!(" ({} items)", count)
            } else {
                "".to_string()
            };

            let dir_prefix = if config.output_format == "markdown" { "📁 **" } else { "[DIR] " };
            let dir_suffix = if config.output_format == "markdown" { "/**" } else { "/" };
            println!("{}{}{}{}", prefix, dir_prefix, name, dir_suffix);
            print_tree(&entry.path, &format!("{}  ", prefix), config, current_depth + 1);
        } else {
            let (size, modified) = (format_size(entry.size), format_modified(entry.modified));
            let ext_info = if let Some(ref ext) = entry.ext {
                format!(".{}", ext)
            } else {
                "".to_string()
            };

            let file_icon = if config.output_format == "markdown" { "📄 " } else { "[FILE] " };
            println!("{}{}{} ({}, {}) [{}]{}",
                     prefix, file_icon, name, size, modified, entry.filetype_desc, ext_info);

            if config.show_content && entry.size <= config.max_file_size && is_text_file(&entry.path) {
                println!();
                if config.output_format == "markdown" {
                    println!("{}  Content:", prefix);
                    let lang = guess_language(&entry.path);
                    println!("{}  ```{}", prefix, lang);
                } else {
                    println!("{}  --- Content Start ---", prefix);
                }
                filter_and_print_content(&entry.path, &config.content_filter, config.content_context, prefix, config.highlight, config.whole_file);
                if config.output_format == "markdown" {
                    println!("{}  ```", prefix);
                } else {
                    println!("{}  --- Content End ---", prefix);
                }
                println!();
            } else if config.show_content && entry.size > config.max_file_size {
                println!("{}  (File not displayed - {})", prefix, size);
            }
        }
    }
}

fn filter_and_print_content(
    path: &Path,
    pattern: &Option<Regex>,
    context: usize,
    prefix: &str,
    highlight: bool,
    whole_file: bool,
) {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => {
            println!("{}    ! Cannot read file", prefix);
            return;
        }
    };

    let reader = io::BufReader::new(file);
    let lines: Vec<String> = reader.lines().filter_map(Result::ok).collect();
    let total_lines = lines.len();

    println!("{}     ┌ Total lines: {}", prefix, total_lines);
    println!("{}     │", prefix);

    match pattern {
        None => {
            for (i, line) in lines.iter().enumerate() {
                format_line(line, i + 1, prefix, None, highlight, false);
            }
            println!("{}     │", prefix);
        }
        Some(regex) => {
            let mut matches = Vec::new();
            for (i, line) in lines.iter().enumerate() {
                if regex.is_match(line) {
                    matches.push(i + 1);
                }
            }

            if matches.is_empty() {
                println!("{}    ! No matches found", prefix);
                println!("{}     │", prefix);
                return;
            }

            if whole_file {
                for (i, line) in lines.iter().enumerate() {
                    let is_match = regex.is_match(line);
                    format_line(line, i + 1, prefix, Some(regex), highlight, is_match);
                }
            } else {
                let context_num = context;
                let mut prev_end = 0;
                for &match_num in &matches {
                    let start = if match_num > context_num { match_num - context_num } else { 1 };
                    let end = std::cmp::min(match_num + context_num, total_lines);

                    if start > prev_end + 1 {
                        print_separator(prefix);
                    }

                    for i in start..=end {
                        let line = &lines[i - 1];
                        let is_match = i == match_num;
                        format_line(line, i, prefix, Some(regex), highlight, is_match);
                    }

                    prev_end = end;
                }
            }
            println!("{}     │", prefix);
        }
    }
}

fn format_line(
    line: &str,
    line_num: usize,
    prefix: &str,
    pattern: Option<&Regex>,
    highlight: bool,
    is_match: bool
) {
    let line_marker = if is_match { "> " } else { "  " };
    let line_num_str = format!("{:4} │{}", line_num, line_marker);

    if highlight && pattern.is_some() && is_match {
        let regex = pattern.unwrap();
        let mut highlighted_line = String::new();
        let mut last_end = 0;
        for mat in regex.find_iter(line) {
            highlighted_line.push_str(&line[last_end..mat.start()]);
            highlighted_line.push_str(&line[mat.start()..mat.end()].yellow().bold().to_string());
            last_end = mat.end();
        }
        highlighted_line.push_str(&line[last_end..]);
        println!("{}{}{}", prefix, line_num_str, highlighted_line);
    } else {
        println!("{}{}{}", prefix, line_num_str, line);
    }
}

fn print_separator(prefix: &str) {
    println!("{}     │", prefix);
    println!("{}   ⋯ │ ...", prefix);
    println!("{}     │", prefix);
}

fn file_type_description(path: &Path) -> String {
    let mime_type = tree_magic_mini::from_filepath(path).unwrap_or("application/octet-stream");
    mime_type.to_string()
}

fn is_text_file(path: &Path) -> bool {
    let mime = tree_magic_mini::from_filepath(path).unwrap_or("application/octet-stream");
    mime.starts_with("text/")
}

fn guess_language(path: &Path) -> String {
    let ext = path.extension().and_then(OsStr::to_str).unwrap_or("");
    match ext {
        "py" => "python",
        "js" => "javascript",
        "jsx" => "jsx",
        "ts" => "typescript",
        "tsx" => "tsx",
        "php" => "php",
        "java" => "java",
        "rb" => "ruby",
        "go" => "go",
        "rs" => "rust",
        "c" => "c",
        "cpp" | "hpp" => "cpp",
        "cs" => "csharp",
        "scala" => "scala",
        "kt" => "kotlin",
        "swift" => "swift",
        "sh" | "bash" | "zsh" => "bash",
        "fish" => "fish",
        "pl" | "pm" | "t" => "perl",
        "css" => "css",
        "scss" => "scss",
        "less" => "less",
        "sql" => "sql",
        "md" => "markdown",
        "json" => "json",
        "xml" => "xml",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "ini" => "ini",
        "conf" => "conf",
        "txt" => "text",
        "csv" => "csv",
        "html" | "htm" => "html",
        "Dockerfile" => "dockerfile",
        "Makefile" => "makefile",
        _ => "text",
    }.to_string()
}

fn is_executable_file(path: &Path) -> bool {
    if let Ok(metadata) = path.metadata() {
        let mode = metadata.permissions().mode();
        mode & 0o111 != 0
    } else {
        false
    }
}

fn matches_type_filter(path: &Path, filters: &[String], is_dir: bool, groups: &HashMap<&str, Vec<&str>>) -> bool {
    if filters.is_empty() {
        return true;
    }

    let name = path.file_name().unwrap_or_else(|| OsStr::new("")).to_string_lossy();
    let metadata = match path.metadata() {
        Ok(m) => m,
        Err(_) => return false,
    };
    let size = metadata.len();
    let file_type = metadata.file_type();

    let is_symlink = file_type.is_symlink();
    let is_socket = file_type.is_socket();
    let is_pipe = file_type.is_fifo();
    let is_block_dev = file_type.is_block_device();
    let is_char_dev = file_type.is_char_device();
    let is_device = is_block_dev || is_char_dev;
    let is_executable = !is_dir && is_executable_file(path);

    let mime = tree_magic_mini::from_filepath(path).unwrap_or("application/octet-stream");
    let is_text = !is_dir && mime.starts_with("text/");

    for filter in filters {
        if filter.starts_with("ext:") {
            let ext_req = &filter[4..];
            let ext = path.extension().and_then(OsStr::to_str).unwrap_or("");
            if ext == ext_req {
                return true;
            }
        } else if filter.starts_with("group:") {
            let group = &filter[6..];
            if let Some(exts) = groups.get(group) {
                let ext = path.extension().and_then(OsStr::to_str).unwrap_or("");
                if exts.contains(&ext) {
                    return true;
                }
            }
        } else {
            match filter.as_str() {
                "binary" => {
                    if !is_dir && !is_text {
                        return true;
                    }
                }
                "text" => {
                    if !is_dir && is_text {
                        return true;
                    }
                }
                "dir" => {
                    if is_dir {
                        return true;
                    }
                }
                "hidden" => {
                    if name.starts_with('.') {
                        return true;
                    }
                }
                "empty" => {
                    if !is_dir && size == 0 {
                        return true;
                    }
                }
                "all" => {
                    return true;
                }
                "socket" => {
                    if is_socket {
                        return true;
                    }
                }
                "pipe" => {
                    if is_pipe {
                        return true;
                    }
                }
                "symlink" => {
                    if is_symlink {
                        return true;
                    }
                }
                "device" => {
                    if is_device {
                        return true;
                    }
                }
                "executable" => {
                    if is_executable {
                        return true;
                    }
                }
                "archive" => {
                    if mime.contains("zip") || mime.contains("x-tar") || mime.contains("x-gzip") {
                        return true;
                    }
                }
                _ => {}
            }
        }
    }

    false
}

fn sort_entries(entries: &mut [DirEntryExt], sort_by: &SortField, direction: &SortDirection, dirs_first: bool) {
    entries.sort_by(|a, b| {
        let dir_cmp = if dirs_first {
            a.is_dir.cmp(&b.is_dir)
        } else {
            std::cmp::Ordering::Equal
        };

        if dir_cmp != std::cmp::Ordering::Equal {
            return dir_cmp;
        }

        let cmp = match sort_by {
            SortField::Name => a.path.file_name().cmp(&b.path.file_name()),
            SortField::Date => a.modified.unwrap_or(UNIX_EPOCH).cmp(&b.modified.unwrap_or(UNIX_EPOCH)),
            SortField::Size => a.size.cmp(&b.size),
            SortField::Type => a.filetype_desc.cmp(&b.filetype_desc),
            SortField::Ext => a.ext.cmp(&b.ext),
        };

        cmp
    });

    if let SortDirection::Desc = direction {
        entries.reverse();
    }
}

fn format_size(size: u64) -> String {
    if size >= 1_073_741_824 {
        format!("{}G", size / 1_073_741_824)
    } else if size >= 1_048_576 {
        format!("{}M", size / 1_048_576)
    } else if size >= 1024 {
        format!("{}K", size / 1024)
    } else {
        format!("{}B", size)
    }
}

fn format_modified(m: Option<SystemTime>) -> String {
    if let Some(time) = m {
        let dt: DateTime<Utc> = time.into();
        dt.to_rfc3339()
    } else {
        "unknown".to_string()
    }
}

