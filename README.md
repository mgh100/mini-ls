# Mini-LS
A simple ls clone written in rust as a learning exercise.  It is intended to meet most of the options of the original
ls over time.  Shared publicly as no reason to keep it private and maybe someone else will find the list of items to 
implement useful as a learning tool.

## Implement the following features
- List the contents of a directory passed in as an argument (I/O, Result, Option, Argument Handling)
- List the contents of the current directory as a default if no argument it passed (N.B current directory command called)
- Print the output to a file when passed the -F argument (I/O, File and Text Formatting)
- Show extended attributes if a file when passed the -l argument (more I/O, Text Formatting)
- Show hidden files when given -A (OS Permissions)
- Do recursive list showing full path from directory root when passed argument -R
- Sort by Size with -S tag 
- Sort by time last modified with the -t (Times and Dates)
- Reverse the sort with r argument appended to -S or -t

## Current Features and Expected Behavior (Current Implementation)

| command          | outcome                                                                                |
|------------------|----------------------------------------------------------------------------------------|
| mini-ls ~/folder | lists all files and directories in the specified folder and prepends each with an icon |
| mini-ls -F out.txt ~/folder | writes the contents of the specified folder out to the file out.txt |
| mini-ls -Fout.txt ~/folder | writes the contents of the specified folder out to the file out.txt |
