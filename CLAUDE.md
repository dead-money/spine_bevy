## Attribution

Do not add `Co-Authored-By: Claude` trailers to commits or "Generated with Claude Code" footers to PR bodies. Author lines and PR bodies stay clean.

## Process

Spine behavior belongs in the sibling `spine_runtime` (`../spine_runtime`), which is ours; this crate is only Bevy glue. CI builds against the runtime's same-named branch when one exists, else `main`, so a change spanning both repos uses matching branch names and merges the runtime PR first. Merge PRs with a merge commit, never squash or rebase.
