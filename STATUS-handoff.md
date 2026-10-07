# STATUS.md handoff template

When the agent is stuck after two real attempts at the same problem, it stops and writes `STATUS.md` in the
repo root with this structure, instead of trying variations at random.

```markdown
# STATUS: <one-line description of the blocker>

Date: YYYY-MM-DD
Step: <step number from docs/DESIGN.md>

## Goal
What this step was supposed to achieve, and its pass condition.

## What happens instead
The symptom, with the exact log lines or error output.

## Attempts
1. What was tried, why it was expected to work, what actually happened (with log evidence).
2. Same for the second attempt.

## What we know for sure
Facts verified by logs or tests.

## What we don't know
Open questions, and how each one could be answered.

## Options from here
Each option with its cost and risk. Mark the one the agent recommends.

## State of the repo
What is committed, what is uncommitted, and whether the game still launches cleanly.
```
