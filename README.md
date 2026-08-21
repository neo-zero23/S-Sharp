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
- **Conditionals**: `if (condition), action.`
- **Loops**: `repeat (count), action.` and `while (condition), action.`
- **Functions**: `define function name(params), return value.`

## Building and Running

```bash
# Build the interpreter
cargo build

# Run the example program
cargo run -- examples/access_control.ssharp
```

## Installation (Windows)

### Option 1: Download the Installer (Recommended)

1. Download the latest `SSharp-Setup.exe` from the [Releases](https://github.com/ssharp-lang/ssharp/releases) page.
2. Run the installer.
3. **Check the box "Add S# to PATH (recommended)"** when prompted.
4. **Open a NEW terminal window** (PowerShell, CMD, or Windows Terminal) — existing terminals won't see the PATH change.
5. Verify the installation:
   ```powershell
   ssharp --version
   # Output: S# (ssharp) v0.1.0
   ```

### Option 2: Build the Installer Yourself

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
├── examples/
│   └── access_control.ssharp
├── src/
│   ├── main.rs              # CLI entry point
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