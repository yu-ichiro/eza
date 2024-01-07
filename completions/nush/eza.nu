export extern "eza" [
    --version(-v)              # Show version of eza
    --help                     # Show list of command-line options
    --oneline(-1)              # Display one entry per line
    --long(-l)                 # Display extended file metadata as a table
    --grid(-G)                 # Display entries in a grid
    --across(-x)               # Sort the grid across, rather than downwards
    --recurse(-R)              # Recurse into directories
    --tree(-T)                 # Recurse into directories as a tree
    --dereference(-X)          # Dereference symbolic links when displaying file information
    --classify(-F)             # Display type indicator by file names
    --color                    # When to use terminal colours
    --colour                   # When to use terminal colours
    --color-scale              # Highlight levels of file sizes distinctly
    --colour-scale             # Highlight levels of file sizes distinctly
    --color-scale-mode         # Use gradient or fixed colors in --color-scale
    --colour-scale-mode        # Use gradient or fixed colors in --colour-scale
    --icons                    # When to display icons
    --no-quotes                # Don't quote file names with spaces
    --hyperlink                # Display entries as hyperlinks
    --absolute                 # Display entries with their absolute path
    --group-directories-first  # Sort directories before other files
    --git-ignore               # Ignore files mentioned in '.gitignore'
    --all(-a)                  # Show hidden and 'dot' files. Use this twice to also show the '.' and '..' directories
    --almost-all(-A)           # Equivalent to --all; included for compatibility with `ls -A`
    --list-dirs(-d)            # List directories like regular files
    --level(-L): string        # Limit the depth of recursion
    --width(-w)                # Limits column output of grid, 0 implies auto-width
    --reverse(-r)              # Reverse the sort order
    --sort(-s)                 # Which field to sort by
    --only-dirs(-D)            # List only directories
    --only-files(-f)           # List only files
    --binary(-b)               # List file sizes with binary prefixes
    --bytes(-B)                # List file sizes in bytes, without any prefixes
    --group(-g)                # List each file's group
    --header(-h)               # When to add a header row to each column
    --links(-H)                # List each file's number of hard links
    --inode(-i)                # List each file's inode number
    --blocksize(-S)            # List each file's size of allocated file system blocks
    --time(-t) -d              # Which timestamp field to list
    --modified(-m)             # Use the modified timestamp field
    --numeric(-n)              # List numeric user and group IDs.
    --changed                  # Use the changed timestamp field
    --accessed(-u)             # Use the accessed timestamp field
    --created(-U)              # Use the created timestamp field
    --time-style               # How to format timestamps
    --total-size               # Show recursive directory size (unix only)
    --no-permissions           # Suppress the permissions field
    --octal-permissions(-o)    # List each file's permission in octal format
    --no-filesize              # Suppress the filesize field
    --no-user                  # Suppress the user field
    --no-time                  # Suppress the time field
    --mounts(-M)               # Show mount details
    --git                      # List each file's Git status, if tracked
    --no-git                   # Suppress Git status
    --git-repos                # List each git-repos status and branch name
    --git-repos-no-status      # List each git-repos branch name (much faster)
    --extended(-@)             # List each file's extended attributes and sizes
    --context(-Z)              # List each file's security context
    --smart-group              # Only show group if it has a different name from owner
    --stdin                    # When piping to eza. Read file paths from stdin
]
