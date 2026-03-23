First rule: Before executing ANY coding tasks, you will update the files ARCHITECTURE.md, PRINCIPLES.md, and README.md as appropriate. If those files don't exist, then this is my first time prompting you about this project, so ask how to make them.

Task tracking uses `bd` (beads). Use `bd list` to see issues, `bd create` to add new ones, `bd update` to modify, and `bd close` to complete. Update task status after every step.

If finishing one task, stop and ask if I'm ready to start the next step instead of just diving in. At the end of every task, double check the files above and bd status to make sure they're up to date.

## Beads Issue Tracker

This project uses **bd (beads)** for issue tracking. Run `bd prime` to see full workflow context and commands.

### Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --claim  # Claim work
bd close <id>         # Complete work
```

### Rules

- Use `bd` for ALL task tracking — do NOT use TodoWrite, TaskCreate, or markdown TODO lists
- Run `bd prime` for detailed command reference and session close protocol
- Use `bd remember` for persistent knowledge — do NOT use MEMORY.md files

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   bd dolt push
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds

# Beads (`bd`) — Agent Instructions

This project uses **Beads** for all issue tracking. Do NOT use markdown TODOs, task lists, or other tracking methods. All work tracking goes through `bd`.

## Your Mental Model

You have no memory between sessions. Beads is your external memory — a structured, queryable database of work items with priorities, dependencies, and audit trails. When you wake up, you have amnesia. Beads tells you where you are.

## Session Start

```bash
bd prime                 # Load workflow context (~1-2k tokens, may run automatically via hooks)
bd ready --json          # See what's unblocked and ready to work on
bd show bd-XXXX --json   # Get full context on a specific issue
```

## Golden Rule: Always Use `--json`

Every `bd` command you run should include `--json` for structured, parseable output.

## Creating Issues

Always include `--description`. Your future self (next session's agent) needs context:

```bash
bd create "Fix authentication bug" \
  --description="Login fails when password contains special characters. Reproduce: try password with single quotes." \
  -t bug -p 1 --json
```

For descriptions with special characters (backticks, nested quotes, `!`, `$variables`), use stdin:

```bash
echo 'Description with `backticks` and "quotes"' | bd create "Title" --description=- --json
```

Or a file:

```bash
bd create "Title" --body-file=description.md --json
```

### Discovered Work

While working on an issue, if you find a new bug or needed work, **always** create a linked issue:

```bash
bd create "Found race condition in cache layer" \
  --description="Cache invalidation not atomic under concurrent writes. Discovered while working on bd-42." \
  --deps discovered-from:bd-42 -t bug -p 1 --json
```

File issues for anything that will take more than ~2 minutes.

## Working on Issues

```bash
bd ready --json                    # Find what's unblocked
bd update bd-a1b2 --claim --json   # Atomically claim it (sets in_progress + assignee)
# ... do the work ...
bd close bd-a1b2 --reason "Fixed: sanitized password input in auth module" --json
```

## Updating Issues

**NEVER use `bd edit`** — it opens an interactive `$EDITOR` you cannot use. Always use `bd update` with flags:

```bash
bd update bd-a1b2 --title "New title" --json
bd update bd-a1b2 --description "Updated description" --json
bd update bd-a1b2 --priority 0 --json
bd update bd-a1b2 --status in_progress --json
bd update bd-a1b2 --design "Architecture notes" --json
bd update bd-a1b2 --notes "Additional context" --json
bd update bd-a1b2 --acceptance "Must pass integration tests" --json
```

## Dependencies

```bash
bd dep add bd-2 bd-1               # bd-2 is blocked by bd-1
bd dep remove bd-2 bd-1            # Remove dependency
bd dep tree bd-2 --json            # View dependency graph
bd dep cycles                      # Detect circular dependencies
bd blocked --json                  # What's stuck?
```

## Labels and Comments

```bash
bd label add bd-42 "backend" --json
bd comment add bd-42 "Reproduced on staging. Root cause is unsanitized input." --json
```

## Session End ("Landing the Plane")

When the user says **"land the plane"** or the session is ending, execute ALL steps below. Do not stop early. Do not say "ready to push when you are." YOU must push.

```bash
# 1. File remaining work as issues
bd create "Remaining task description" -t task -p 2 \
  --description="Context for next session" --json

# 2. Run quality gates (if code changes were made)
#    Run tests and linting. File P0 issues for any failures.

# 3. Close finished issues
bd close bd-42 --reason "Completed" --json

# 4. PUSH TO REMOTE — MANDATORY, DO NOT SKIP
git pull --rebase
git push
git status              # Must show "up to date with origin/main"

# 5. Clean up git state
git stash clear
git remote prune origin

# 6. Verify clean state
git status

# 7. Find next work and generate handoff prompt
bd ready --json
```

Then provide the user with:
- Summary of what was completed this session
- What issues were filed for follow-up
- Confirmation that everything is pushed to remote
- A handoff prompt: "Continue work on bd-X: [title]. [What's done and what's next]"

**The plane is NOT landed until `git push` succeeds.** Unpushed work causes severe rebase conflicts in multi-agent workflows.

## Quick Command Reference

| Task | Command |
|------|---------|
| Load context | `bd prime` |
| See ready work | `bd ready --json` |
| Create issue | `bd create "Title" --description="..." -t type -p N --json` |
| Show issue | `bd show bd-42 --json` |
| List open issues | `bd list --status open --json` |
| Claim work | `bd update bd-42 --claim --json` |
| Close issue | `bd close bd-42 --reason "..." --json` |
| Add dependency | `bd dep add bd-child bd-parent` |
| View dep tree | `bd dep tree bd-42 --json` |
| Find blocked | `bd blocked --json` |
| Add label | `bd label add bd-42 "label" --json` |
| Add comment | `bd comment add bd-42 "text" --json` |
| Sync | `bd sync` |
| Health check | `bd doctor` |

## What NOT to Do

- **Never use `bd edit`** — opens interactive editor you can't use
- **Never use markdown TODOs** — all tracking goes through `bd`
- **Never skip sync/push** at session end — causes data loss
- **Never create issues without `--description`** — future agents need context
- **Never forget `--json`** — always use structured output

## Commit Messages

Include issue IDs for traceability:

```bash
git commit -m "Fix auth validation bug (bd-a1b2)"
```

## Recovery

```bash
bd doctor              # Diagnose problems
bd doctor --fix        # Auto-repair common issues
bd info --json         # System info and schema
```
