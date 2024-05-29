complete -f -c c3 -d "Performance mode, don't read dependencies" -s 'n' -l 'no-tree'
complete -f -c c3 -d "List todos (non interactive)" -s 'l' -l 'list'
complete -f -c c3 -s "d" -l "show-done" -d "Show done todos too"
complete -f -c c3 -s "m" -l "enable-module" -d "Enable TUI module at startup"
complete -f -c c3 -s "s" -l "stdout" -d "Write contents of todo file in the stdout (non interactive)"
complete -rF -c c3 -d "Path to todo file (and notes sibling directory) [default: /home/nima/.local/share/calcurse/todo]"
complete -x -c c3 -s "h" -l "help" -d "Print help"
complete -x -c c3 -s "V" -l "version" -d "Print version"
complete -x -c c3 -s "M" -l "minimal-tree" -d "Minimal tree with no tree graphics"
complete -x -c c3 -s "S" -l "search-and-select" -d "Search and select todo. Used for batch change operations" -r
complete -x -c c3 -s "a" -l "append-todo" -d "Append todo" -r
complete -x -c c3 -s "A" -l "prepend-todo" -d "Prepend todo" -r
complete -f -c c3 -l "set-selected-priority" -d "Set selected todo priority" -a "0 1 2 3 4 5 6 7 8 9" -r
complete -f -c c3 -l "set-selected-message" -d "Set selected todo message" -r
complete -x -c c3 -l "delete-selected" -d "Delete selected todos"
complete -x -c c3 -l "done-selected" -d "Done selected todos"
complete -x -c c3 -l "done-string" -d 'String before done todos [default: "[x] "]'
complete -x -c c3 -l "undone-string" -d 'String before undone todos [default: "[ ] "]'
