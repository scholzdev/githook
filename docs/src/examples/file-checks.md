# File Checks

Block certain file types:

```ghook
let forbidden = ["exe", "dll", "zip"]

foreach git.staged_files {
    file in
    let is_forbidden = forbidden.any(ext => file.extension == ext)
    
    block if is_forbidden
        message "File type not allowed: ${file.name}"
}
```

Check file size:

```ghook
foreach git.staged_files {
    file in
    let mb = (file.size / 1024 / 1024).round()
    
    warn if mb > 1
        message "Large file: ${file.name} (${mb}MB)"
    
    block if mb > 5
        message "File too large: ${file.name}"
}
```

Filter by extension:

```ghook
let rust_files = git.staged_files.filter(f => f.extension == "rs")
let test_files = git.staged_files.filter(f => f.name.contains("test"))

run "echo Rust files: ${rust_files.length}"
run "echo Test files: ${test_files.length}"
```

Check file content:

```ghook
foreach git.staged_files {
    file in
    
    warn if file.content.contains("TODO")
        message "TODO found in ${file.name}"
    
    block if file.content.contains("password =")
        message "Hardcoded password in ${file.name}"
}
```
