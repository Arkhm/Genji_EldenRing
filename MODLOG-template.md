# MODLOG entry template

Copy this block to the **top** of `MODLOG.md` after every change.

```markdown
## YYYY-MM-DD: <step number and short title>

**What changed:** one or two sentences, plain language.

**Files:** the files added or changed.

**How to test:**
1. exact command to run
2. what to do in game
3. what you should see, and which log lines prove it

**Result:** passed / failed / not tested. Who tested and how. Paste the key log lines.

**Dead ends:** what was tried and didn't work, and why. Write "none" if there were none.

**Undo:** how to revert this step (usually `git revert <commit>`), plus anything outside git to undo.
```
