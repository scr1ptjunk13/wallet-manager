#!/bin/bash

# Check if a file is provided as an argument
if [ $# -ne 1 ]; then
  echo "Usage: $0 <python_file>"
  exit 1
fi

# Check if the file exists
if [ ! -f "$1" ]; then
  echo "Error: File '$1' not found"
  exit 1
fi

# Extract function names using grep and awk
grep -E '^[[:space:]]*def[[:space:]]+[a-zA-Z_][a-zA-Z0-9_]*\(' "$1" |
  awk '{gsub(/^[ \t]*def[ \t]+/, ""); gsub(/\(.*$/, ""); print}' |
  sort | uniq

exit 0
