# MODLOG entry template

Copy this block to the **top** of `MODLOG.md` after every change. Log everything that changed, including
changes outside the repo (game folder, saves, installed tools), and exactly what was tested.

```markdown
## YYYY-MM-DD Short title

**Changed:** what was changed, and in which files
**Why:** the problem or goal
**Tested how:** what you did to check it (played the game, read logs, ran a test)
**Result:** what happened, with numbers or log lines where possible
**Still broken / not tested:** be honest
**Next:** what to do next
```

## Example

```markdown
## 2026-10-03 Player position sync

**Changed:** Added position messages from the gameplay game to the host plugin (`mod/LinkReader.java`, `plugin/link.cpp`)
**Why:** Step 2 of the plan: send one piece of data between the games
**Tested how:** Started both games, walked around, compared the position each side logged
**Result:** Positions match within about one frame at normal walking speed
**Still broken / not tested:** Fast travel and loading screens not tested; no rotation yet
**Next:** Send collision shapes the other way
```
