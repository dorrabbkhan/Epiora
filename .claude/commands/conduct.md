---
description: Run one plan task end to end (task → review ∥ CI → judge → fixes → status), never merging.
argument-hint: <version> <task-number>   e.g. v0.1 2
---
Run the conductor loop from CLAUDE.md for plan version $1, task $2.
1. Read the plan file for $1 (mapping in .devin/skills/epiora-task/SKILL.md), its Global Constraints, its
   Review Focus, its Task Index and Task $2; the spec sections it names; docs/conductor/standing-notes.md; and
   docs/conductor/notes/$1/task$2.md if it exists.
2. Check that the dependencies in the Task Index are merged; if not, say so and stop. If the Task Index marks
   the task OWNER GATE or OWNER REVIEW, run it but stop for the owner before declaring it done.
3. Build the task prompt: "/epiora-task $1 $2" + standing notes + per-task notes + footer, and run it with
   scripts/conductor/devin.sh task $1 $2.
4. Follow the loop and judgment rules exactly. Keep a short running log in
   ~/epiora-conductor/$1-task$2/conductor-log.md.
5. Finish with scripts/conductor/status-comment.sh and scripts/conductor/notify.sh, then print a summary:
   PR, verdict, CI, fix rounds used, anything the owner must look at (gates, human steps, decisions).
