complete -f -c c3 -d "Performance mode, don't read dependencies" -s 'n' -l 'no-tree'
complete -f -c c3 -d "List todos (non interactive)" -s 'l' -l 'list'
complete -f -c c3 -s "d" -l "show-done" -d "Show done todos too"
complete -f -c c3 -s "m" -l "enable-module" -d "Enable TUI module at startup"
complete -f -c c3 -s "s" -l "stdout" -d "Write contents of todo file in the stdout (non interactive)"
complete -rF -c c3 -s "p" -l "todo-path" -d "<TODO_PATH> Path to todo file (and notes sibling directory) [default: /home/nima/.local/share/calcurse/todo]"
complete -x -c c3 -s "h" -l "help" -d "Print help"
complete -x -c c3 -s "V" -l "version" -d "Print version"
complete -c c3 -f
