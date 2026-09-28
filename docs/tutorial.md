# S# Tutorial — from zero to FizzBuzz

S# reads like English sentences: a **period `.`** ends a statement,
a **comma `,`** separates clauses. No braces, no semicolons.

Run a file with `ssharp hello.ssharp`, or type `ssharp` alone for the
interactive REPL (every fragment must end with `.`, `:quit` exits).

## 1. Hello, display, comments

```ssharp
when (start_clicked).
# `#` starts a comment.
display "Hello, world.".
```

## 2. Variables and input

```ssharp
when (start_clicked).
ask "How old are you?" and save to age.
save age + 1 to next.
display "Next year: " ++ next.
```

Values have types — check with `type(x)`: `int` (`42`), `float`
(`3.5`), `string`, `bool`, `list`. Literals without a decimal point
are ints: `7 / 2` is `3.5` (float division), `7 div 2` is `3`.

## 3. Conditions

```ssharp
when (start_clicked).
ask "How old are you?" and save to age.
if (age >= 18), display "granted", else display "denied".
if (age < 13), display "child", else if (age < 18), display "teen", else display "adult".
```

Comparison is `==` (`=` alone is an error), plus `!=`, `>`, `<`,
`>=`, `<=`. Combine with `and`, `or`, `not` and the literals
`true` / `false`.

## 4. Loops, break, continue

```ssharp
when (start_clicked).
repeat (3), display "hi".
save 0 to i.
while (i < 5), save i + 1 to i, display i.
for each n in range(5), display n.
```

`break` exits a loop, `continue` skips to the next round:

```ssharp
when (start_clicked).
for each n in range(10), if (n == 3), continue, else display n.
save 0 to i.
while (true), save i + 1 to i, if (i == 3), break.
```

Pattern to remember: inside a loop, code after an `if` goes in
`else` — `if (c), A, B.` parses `B` as part of the then-branch.

## 5. Lists

```ssharp
when (start_clicked).
save ["Ada", "Grace"] to heroes.
add "Linus" to heroes.
change item 1 of heroes to "ADA".
display item len(heroes) of heroes.
display len(heroes).
remove item 2 of heroes.
```

Indexing is 1-based, Scratch-style: `item 1 of xs` is the first
element. It also works on strings: `item 1 of "hey"` is `"h"`.
`range(4)` is `[1, 2, 3, 4]` — 1-based on purpose.

## 6. Functions and modules

```ssharp
when (start_clicked).
define function double(n), return n * 2.
define function sum(xs), save 0 to total, for each x in xs, save total + x to total, return total.
display sum([1, 2, 3]).
```

A body is comma-separated actions ending with `return` (mandatory —
S# has no null, so a missing `return` is a compile error).
Assignments inside stay local. Share code between files — the path
resolves relative to the importing file, and each file runs once:

```ssharp
when (start_clicked).
import "string_utils.ssharp".
display shout("hello").
```

## 7. Capstone: FizzBuzz

```ssharp
when (fizzbuzz).
for each n in range(15), if (n % 15 == 0), display "FizzBuzz", else if (n % 3 == 0), display "Fizz", else if (n % 5 == 0), display "Buzz", else display n.
```

## 8. Files and errors

```ssharp
when (files).
write "line1\nline2" to file "out.txt".
read "out.txt" and save to content.
display content.
try read "missing.txt" and save to c, catch e, display e.
```

`try A, catch B.` recovers from runtime errors (bad index, missing
file, division by zero). With `catch e, ...`, `e` holds the message.
Parse errors are never caught — they are bugs, fix them.

## 9. Keep it tidy

```bash
ssharp fmt messy.ssharp          # print canonical form (one statement per line)
ssharp fmt --write messy.ssharp  # reformat in place
```

`./install.sh` also sets up VS Code and Kate syntax highlighting
from `editors/`. Full keyword list: see "Reserved words" in README.
