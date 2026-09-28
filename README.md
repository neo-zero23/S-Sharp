# S# Language Interpreter

![S# Logo](logos/s-sharp.svg)

**S#** (pronounced "S-sharp") is an educational programming language designed with the philosophy: **"Scratch in text form"**.

## Philosophy

S# aims to feel like reading natural English sentences. It avoids the visual noise of traditional programming languages — no curly braces `{}`, no colons `:`, no semicolons `;`. Instead, it uses punctuation that mirrors English grammar:

- **Period `.`** ends a statement (like a sentence)
- **Comma `,`** separates clauses within a statement
- **Parentheses `()`** group expressions and conditions

## Syntax Example

```ssharp
when (start_clicked).
ask "How old are you?" and save to age.
if (age >= 18), display "Access granted".
if (age < 18), display "Access denied".
```

## Language Features

- **Events**: `when (event_name).` — program entry point
- **Variables**: `save <expression> to <name>.` or `<expr> and save to <name>.`
- **Input/Output**: `ask "prompt"`, `display <value>`
- **Conditionals**: `if (condition), action.` and `if (condition), action, else other_action.` (`else if` chains supported)
- **Loops**: `repeat (count), action.`, `while (condition), action.` and `for each x in xs, action.` — iterates lists element by element; iterating a string yields one single-character string per step (`for each c in "hey", display c.` prints `h`, `e`, `y`)
- **Loop control**: `break.` exits the loop, `continue.` skips to the next round. Inside a loop, code after an `if` goes in `else`: `for each n in range(10), if (n == 3), continue, else display n.`
- **Functions**: `define function name(params), action1, action2, return value.` — body actions run in an isolated scope, then the `return` expression is evaluated. The single-expression form still works: `define function double(n), return n * 2.` The final `return` is mandatory: S# has no null value, so a function without `return` is a parse-time error, not a silent nothing.
- **Lists**: `[1, "two", true]` literals, 1-based access `item 2 of xs` (works on strings too: `item 1 of "hey"` is `"h"`), length with `len(xs)` / `len("hey")`
- **List mutation**: `add 4 to xs.` (push), `change item 1 of xs to 10.`, `remove item 2 of xs.`
- **Booleans**: `true`, `false`, `not`, `or`, `and` (e.g. `if (true and not false), display "yes".`)
- **Comparison**: `==`, `!=`, `>`, `<`, `>=`, `<=` (note: single `=` is a lex error — use `==`)
- **Arithmetic**: `+`, `-`, `*` (int-preserving with overflow checks), `/` (always float: `7 / 2` is `3.5`), `div` (truncating integer division: `7 div 2` is `3`), `%` (modulo, exact on ints)
- **Builtins**: `len(x)`, `range(n)`, `str(x)`, `num(x)`, `upper(s)`, `lower(s)`, `split(s, sep)`, `join(list, sep)`, `sqrt(n)`, `type(x)` (`int` / `float` / `string` / `bool` / `list`) — note `range` is 1-based on purpose: `range(4)` is `[1, 2, 3, 4]`, so `item i of range(n)` lines up with Scratch-style indexing instead of Python-style `0..n-1`
- **Error handling**: `try risky, catch display "fallback".` or with the message bound: `try risky, catch e, display e.` — catches runtime errors only (bad index, missing file, division by zero); parse errors are programmer errors and abort
- **Modules**: `import "utils.ssharp".` — runs the file once (repeat imports and cycles are skipped), paths resolve relative to the importing file; the file's functions and variables become available to the importer
- **File I/O**: `read "data.txt" and save to content.` and `write content to file "out.txt".` (overwrite; missing files are runtime errors, so they work with `try/catch`; single reads capped at 10 MiB)
- **Concatenation**: `++` always joins text (`1 ++ 2` is `"12"`), while `+` adds numbers (`1 + 2` is `3`)
- **Comments**: `#` starts a comment until end of line
- **Strings**: escape sequences `\n`, `\t`, `\r`, `\"`, `\\`

## Reserved words

These cannot be used as variable or function names:

```
when ask save to and display if else repeat while define function return
true false or not item of add change remove for each in
try catch import read write file break continue div
```

## Formatter, editors, tutorial

```bash
ssharp fmt messy.ssharp          # print canonical form: one statement per line
ssharp fmt --write messy.ssharp  # reformat the file in place
```

`fmt` parses before formatting and refuses broken code; comments and
strings (even `"periods."` and decimals like `3.14`) survive untouched.
Syntax highlighting ships in `editors/` (VS Code + Kate, installed by
`./install.sh`). New to S#? Start with [docs/tutorial.md](docs/tutorial.md).

## Building and Running

```bash
# Build the interpreter
cargo build

# Run an example program
cargo run -- examples/access_control.ssharp

# Start the interactive REPL (no file argument)
cargo run
```

Inside the REPL every fragment must end with `.`, variables and
functions persist between fragments, and fragments may span several
lines. Commands: `:help`, `:reset`, `:quit` (Ctrl-D also exits).

## Installation

### Option 1: install.sh (Linux / macOS, recommended)

```bash
git clone https://github.com/neo-zero23/S-Sharp.git
cd S-Sharp
./install.sh
```

The script compiles the interpreter (`cargo build --release`), copies it
to `~/.local/bin/ssharp`, checks that `~/.local/bin` is in your `PATH`,
registers the `.ssharp` file type with your editor (Kate, VS Code) via
`xdg-mime`, and installs the `S# REPL` entry in the applications menu.

If `~/.local/bin` is not in your `PATH`, add this line to your
`~/.bashrc` or `~/.zshrc` and open a new terminal:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

### Option 2: PKGBUILD (Arch Linux / CachyOS)

```bash
git clone https://github.com/neo-zero23/S-Sharp.git
cd S-Sharp
makepkg -si
```

This installs `ssharp` to `/usr/bin`, plus docs, examples and the
`S# REPL` menu entry.

### Option 3: Build from source (any platform with Rust)

```bash
cargo build --release
./target/release/ssharp examples/hola.ssharp
```

### Option 4: Windows installer (.exe)

### Option 4.1: Download the Installer (Recommended)

1. Download the latest `SSharp-Setup.exe` from the [Releases](https://github.com/ssharp-lang/ssharp/releases) page.
2. Run the installer.
3. **Check the box "Add S# to PATH (recommended)"** when prompted.
4. **Open a NEW terminal window** (PowerShell, CMD, or Windows Terminal) — existing terminals won't see the PATH change.
5. Verify the installation:
   ```powershell
   ssharp --version
   # Output: S# (ssharp) v0.1.0
   ```

### Option 4.2: Build the Installer Yourself

If you want to build the installer from source:

1. Install [Inno Setup Compiler](https://jrsoftware.org/isinfo.php) (free).
2. **Convert a logo to `.ico` format** (required for the installer icon):
   - The `logos/` folder contains PNG/SVG files.
   - Use an online converter like [convertio.co](https://convertio.co/png-ico/) or [icoconvert.com](https://icoconvert.com/).
   - Convert `logos/256x256.png` → save as `logos/ssharp.ico`.
3. Open `installer.iss` in Inno Setup Compiler (right-click → "Compile" or File → Open).
4. Click **Build** → the installer `SSharp-Setup.exe` will be created in the `Output/` folder.
5. Run the generated installer and follow steps 2-5 from Option 1.

### Usage After Installation

```powershell
# Run an S# script
ssharp path\to\script.ssharp

# Show help
ssharp --help

# Show version
ssharp --version
```

## Project Structure

```
.
├── logos/                    # S# brand assets (SVG/PNG)
├── Cargo.toml
├── install.sh                # Linux/macOS installer (-> ~/.local/bin)
├── PKGBUILD                  # Arch Linux / AUR package
├── ssharp.desktop            # "S# REPL" menu entry
├── installer.iss             # Windows installer (Inno Setup)
├── examples/
│   └── access_control.ssharp
├── src/
│   ├── main.rs              # CLI entry point (+ REPL)
│   ├── lib.rs               # Library exports
│   ├── error.rs             # Unified error types
│   ├── lexer/               # Hand-written lexer
│   ├── parser/              # Recursive-descent parser
│   └── interpreter/         # Tree-walking evaluator
└── tests/
    └── integration_test.rs  # End-to-end tests
```

## License

MIT