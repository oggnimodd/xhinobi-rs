# xhinobi-rs

`xhinobi-rs` is a command-line tool for aggregating text content from multiple files. It can prepend file names, minify output, ignore specified files, display a directory tree, and remove comments from code.

## Installation

To install `xhinobi-rs`, you need to have the Rust toolchain installed. You can install it from [rustup.rs](https://rustup.rs/).

Once you have Rust installed, you can clone this repository and build the project:

```bash
git clone https://github.com/your-username/xhinobi-rs.git
cd xhinobi-rs
cargo build --release
```

The executable will be located at `target/release/xhinobi`.

## Usage

`xhinobi-rs` reads a list of file paths from standard input. You can use it with `find` or other commands that produce a list of files.

### Basic Usage

To aggregate the content of all files in the current directory, you can use:

```bash
find . -type f | xhinobi
```

### Options

-   `-n`, `--prependFileName`: Prepend the file name before the content of each file.
-   `-m`, `--minify`: Minify the output by removing extra whitespace.
-   `-i`, `--ignore <PATTERN>`: Ignore files matching the specified glob pattern. This option can be used multiple times.
-   `-t`, `--tree`: Prepend the output with a directory tree (requires the `tree` command to be installed).
-   `-o`, `--osc52`: Use OSC52 escape sequence for clipboard over SSH.
-   `-d`, `--decomment`: Remove comments from files using tree-sitter. This feature supports TypeScript, JavaScript, JSON, Python, Rust, Go, Bash, and PHP.

### Examples

**Prepend file names:**

```bash
find . -type f | xhinobi -n
```

**Ignore `node_modules` and `target` directories:**

```bash
find . -type f | xhinobi -i "**/node_modules/**" -i "**/target/**"
```

**Show a directory tree and remove comments:**

```bash
find . -type f | xhinobi -t -d
```
