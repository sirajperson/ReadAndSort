![rs](rs.jpg)

# rs

**rs** is a command-line utility that maps and displays a directory tree with syntax highlighting, file-type filtering, sorting, and optional content display. It’s designed to help developers explore project directories efficiently, especially when working with large codebases or language models.

## Features

- **Directory Mapping:**  
  Recursively prints a directory’s structure, controlled by a configurable depth.
  
- **File Type Filtering:**  
  Filter results by file extension, file type groups (e.g. `group:web`), or special attributes (e.g. `binary`, `dir`, `executable`).
  
- **Sorting:**  
  Sort entries by name, date, size, type, or extension. Supports ascending or descending order, and optionally list directories first.
  
- **Content Display:**  
  Show file contents inline, with optional pattern matching. Highlight matches, display context lines, or show the entire file if a match is found.
  
- **Multiple Formats:**  
  Output as Markdown or plain text.
  
- **Multiple Filters:**  
  Specify multiple `-t` (type) filters by repeating the flag (e.g. `-t ext:py -t group:web`) to broaden your search criteria.

## Installation

You’ll need [Rust](https://www.rust-lang.org/tools/install) installed. Then:

```bash
git clone https://github.com/yourusername/rs.git
cd rs
cargo build --release


The binary will be in target/release/rs. You can move it to a directory in your PATH for easy use:

bash
Copy code
mv target/release/rs /usr/local/bin/
Usage
bash
Copy code
rs [OPTIONS] [directory]
If no directory is specified, it defaults to the current directory (.).

Examples
List top-level entries in ./src:

bash
Copy code
rs ./src
Show Python files 3 levels deep and include file contents:

bash
Copy code
rs -d 3 -t ext:py -c
Show code files sorted by modification date:

bash
Copy code
rs --sort date -t group:code
Show files containing "TODO" and highlight matches:

bash
Copy code
rs -c -p "TODO" --highlight
Filter by more than one type (Python files OR files in the web group):

bash
Copy code
rs -t ext:py -t group:web ./src
No depth limit (unlimited recursion):

bash
Copy code
rs -d 0
Options
-d, --depth N
Maximum directory depth (default: 1, 0 = unlimited)

-f, --format FMT
Output format (markdown or text, default: markdown)

-e, --exclude P
Exclude directories or files by name (can be repeated)

-c, --content
Show file contents in the tree

-s, --max-size N
Maximum file size in bytes for content display (default: 100000)

-t, --type T
Filter by type (can be repeated). Use ext:EXT, group:GROUP, or special types like binary, text, dir, etc.

-p, --pattern PAT
Show only content matching a given regex pattern

--context N
Show N lines of context around matches (default: 0)

--whole-file
Show the entire file if any line matches

--highlight
Highlight matching content

--sort FIELD
Sort by name,date,size,type,ext (default: name)

--direction DIR
Sort direction: asc or desc (default: asc)

--dirs-first
Show directories first (default: true)

--no-dirs-first
Don’t sort directories separately

-h, --help
Show help message

Type Filters
File Types:

ext:EXT — Show files with a specific extension (e.g., ext:py)
group:GROUP — Show files from a specific group (e.g., group:web)
Special Types:

binary — Show binary files
text — Show text files
dir — Show directories
socket — Show sockets
pipe — Show pipes
executable — Show executable files
symlink — Show symbolic links
device — Show device files
hidden — Show hidden files
empty — Show empty files
archive — Show archive files
Contributing
Contributions are welcome! If you have ideas, bug reports, or feature requests, please open an issue or submit a pull request.

License
This project is distributed under the MIT License. See LICENSE for details.

