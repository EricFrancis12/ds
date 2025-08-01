# Directory Stats (ds)

A simple command-line utility written in Rust that displays the sizes of files and directories using horizontal bar charts.

It allows sorting by file name or size.

<img src="https://github.com/user-attachments/assets/43a50a67-6a63-4f99-b4fd-126ec2ccd21c" />

## Features

📁 Recursively calculates sizes of files and directories

🔤 Sort output by file name or file size

📊 Visual ASCII bar chart representation in terminal

## Installation

```bash
cargo install --git https://github.com/EricFrancis12/ds
```

## Usage

```bash
ds [OPTIONS] [DIR]
```

## Arguments

- `[DIR]`: (optional) Path to the target directory. Defaults to the current directory.

## Options

- `-n, --name`: Sort entries alphabetically by name
- `-s, --size`: Sort entries by size (largest first)

## Example Usage

```bash
# Run in current directory (default)
ds

# Analyze a specific directory
ds /path/to/dir

# Sort by name
ds -n /path/to/dir

# Sort by size
ds -s /path/to/dir
```
