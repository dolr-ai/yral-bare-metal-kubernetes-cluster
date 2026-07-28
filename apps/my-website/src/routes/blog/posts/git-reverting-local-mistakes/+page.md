---
title: "Reverting local mistakes in code using Git revert, rebase, restore and reset"
description: "Using git to modify local history to revert or reset to a previous state in history to fix or remove commit errors"
author: "Saikat Das"
tags:
  - git
  - version-control
  - revert
  - rebase
  - restore
  - reset
icon: "git.png"
date: "2021-01-23"
---

We all make mistakes while programming and we tend to only realize it after we've made some more mistakes. 🙄 However, our friend Git is there for us and we can use Git to revert or modify our repository history to get to a desirable state.

The gotcha to remember is that most of these should be applied to our local branch that has not yet been committed. If we make changes to a branch that has been committed and force push our modified history to it, it will break repositories for other people who are tracking the same repository. And they'll not like us anymore. 😭

## Remove all local changes in a file or undelete a file

```bash
git restore <filename>
```

## Restore patches/chunks in a changed file

```bash
git restore -p <filename>
```

## Remove all local changes in the branch

```bash
git restore .
```

## Fixing the last commit

```bash
git commit --amend -m "correct commit"
```

Here, we include whatever changes we missed to make git include those changes in the last commit. The `-m` flag lets us change the commit message

## Removing changes from a commit in the middle

```bash
git revert <commit_hash>
```

This creates a new commit that has the exact opposite changes from the commit we are reverting and applying it leaves us with a repository where the changes from the old commit have been negated by the new commit

## Reset branch to an old commit state

```bash
git reset --hard <commit_hash>
git reset --mixed <commit_hash>
```

- The `--hard` flag resets it to the specified commit and removes all the changes after that point.
- The `--mixed` flag resets the branch to the old state but keeps the changes in the tracked files in case we want to keep some of the newer changes

## Reset a particular file to an old commit state

```bash
git restore --source <commit_hash> <filename>
```

## Undelete deleted/reset commits by using `reflog`

```bash
git reflog
git branch <new_branch_name> <deleted_commit_hash_to_branch_from>
```

`git reflog` shows us a journal of the git commands run on a repository including deletions and we can use it to find the commit hash of deleted commits that we might want to restore

## Get the deleted changes from a deleted branch

```bash
git reflog
git branch <new_branch_name> <commit_hash_at_which_branch_was_deleted>
```

## Moving a commit to a new branch and removing the changes from the old branch

```bash
git branch <new_branch_name>
git reset HEAD~1 --hard
```

This creates the new branch with the changes included and resets the original branch to the penultimate commit state. We can define how many commits we want to move back by relative to the HEAD by specifying a number with the `~` sign.
Ideally, this kind of scenario is encountered when you want to revert committed changes from the long running branch like `master` or `development` and move those changes to a `feature branch`

## Moving a commit to an existing branch and removing the change from the old branch

```bash
git checkout <destination_branch_name>
git cherry-pick <commit_hash_to_be_moved>
git checkout <source_branch_name>
git reset HEAD~1 --HARD
```

## Rewriting commit history using **interactive rebase**

```bash
git rebase -i HEAD~<number_of_commits_to_move_back_by>
git rebase -i <commit_hash_to_move_back_to>
```

This determines which actions we want to perform. Once confirmed we'll get subsequent prompts with options to change the history.
Some of the possible options are:
- `reword` - this lets us rewrite old commit messages
- `drop` - remove an old commit
- `squash` - can be used to combine multiple commits into one. Get combined with the last commit that isn't being squashed
- `fixup` - can be used to edit old commits. Both changes in files as well as files included in commits can be edited