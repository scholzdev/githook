#!/bin/bash

# pre-commit hook - traditional bash implementation
# This would normally be in .git/hooks/pre-commit

# Get branch name
BRANCH_NAME=$(git rev-parse --abbrev-ref HEAD)

# Macro equivalent: block_main
block_main() {
    if [ "$BRANCH_NAME" = "main" ]; then
        echo "❌ Direct commits to main are not allowed."
        exit 1
    fi
}

# Print branch name
echo "$BRANCH_NAME"

# Call the macro equivalent
block_main

# Get all staged files
STAGED_FILES=$(git diff --cached --name-only --diff-filter=ACM)

# Check if there are any staged files
if [ -z "$STAGED_FILES" ]; then
    echo "No staged files"
    exit 0
fi

# Iterate through each staged file
while IFS= read -r file; do
    echo "$file"
    
    # Get file extension
    extension="${file##*.}"
    
    # Match on extension
    if [ "$extension" = "toml" ]; then
        echo "toml file staged: $file"
    else
        echo "Non-toml file staged: $file"
    fi
    
done <<< "$STAGED_FILES"

# Exit with success
exit 0
