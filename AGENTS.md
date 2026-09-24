# Wrath CLI

This is the Wrath CLI, a simple command-line tool to annihilate any process written in Rust. It provides within a single command the ability to kill any process.

## What it does

### User story 1

As a user,
I keep running into an issue;
the port I want to take is already in use,
so I want to kill the process that is using that port.

User uses the command `wrath 3000` to kill the process that is using port 3000.

Wrath CLI is able to recognize the user's intent to kill the process using the port 3000.
It then kills the process using the port 3000 and all of its children.
It then prints the following message:
`Wrath unleashed: Process '<process_name>' (n process(es)) using port 3000 has been obliterated.`
next to the flame (see "Output style").

### User story 2

As a user,
I want to kill a process by its name,
so I can stop it from running.

User uses the command `wrath <process_name>` to kill the process for any reason they have.

Wrath CLI is able to recognize the user's intent to kill the process named <process_name>.
It then kills the process <process_name> and all of its children.
It then prints the following message:
`Wrath unleashed: Process '<process_name>' (n process(es)) has been obliterated.`
next to the flame (see "Output style").

## Output style

- No emojis: only ASCII or plain Unicode characters.
- Line markers: `[>]` found, `[?]` question, `[!]` warning, `[x]` error, `[-]` aborted, `[i]` info.
- The final message is printed next to a Unicode block flame modeled on the fire emoji
  (drawing in `FLAME`, colors painted by hand in `FLAME_COLORS` in `src/ui.rs`:
  `R` red, `O` orange, `Y` yellow core drawn on an orange background):

```
    ▙
  ▟▖▟█▖
 ▟█████
▐██▗▖██▌   Wrath unleashed: Process 'node' (3 processes)
 ▀███▛▘    using port 3000 has been obliterated.
```

## Code standards

- Use as much pure Rust code as possible.
- Keep jobs separate.
- No linting errors or warnings.
- At least 60% test coverage.
- Use `clap` to manage command-line arguments.
- Use `sysinfo` to manage processes and ports.
- Always use the latest crate versions.
- Use `cargo fmt` to format your code.
- Use `cargo clippy` to lint your code.
- Use `cargo test` to test your code.
- Use `cargo audit` to check for security vulnerabilities.
- Use `cargo outdated` to check for outdated dependencies.
- Keep IO operations to a minimum.
- Keep the code as simple as possible.
- Keep the code as readable as possible.
- Keep the code as maintainable as possible.
- Keep the code as performant as possible.
- Do not use unsafe code.
- Do not use `unsafe` blocks.
- Be sure to handle errors gracefully.
- Be sure to handle edge cases.
- Be sure to handle all possible inputs.

## Instructions

At the end of the task you were given, ALWAYS ask the following question using opencode's built-in question system:
`Do you want to continue this subtask?` with only one option: `No`. This is mandatory. Every request HAS to end with this question.
