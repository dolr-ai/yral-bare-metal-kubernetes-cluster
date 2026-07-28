---
title: "Using Git branches effectively: Creating, switching, deleting, tracking, renaming, publishing, merging, rebasing and comparing"
description: "Using branching in Git effectively. Performing most of the common operations that are part of Git's branching strategies"
author: "Saikat Das"
tags:
  - git
  - version-control
  - reference
  - branching
  - tracking
  - merging
  - rebasing
icon: "git.png"
date: "2021-07-27"
---

Git is pretty revolutionary in how its branching system is so lightweight and lets us create potentially unlimited branches in our source code repository to play around and experiment with it. 🥰

Git branches can be understood in terms of **local** branches, i.e., branches that reside on our local repository on disk and **remote** branches, i.e., branches that reside on a remote computer which we use as a source of truth for syncing and exchanging source code with our peers who also have a copy of this repository locally and are working on it.

When working with branches, we need to be aware of the **HEAD** branch as this tracks the current state of the repository and lets us perform different branching operations on the repository. And this is part of our local branch system.

## Create a new branch

```bash
# This will create a branch at the same commit as our HEAD branch
git branch <new_branch_name>

# This will create a branch at the specified commit
git branch <new_branch_name> <commit_hash>
```

## Switch branches

```bash
# checkout has other utilities apart from switching branches
git checkout <branch_name>

# switch is newer syntax. designed to clearly communicate intent
git switch <branch_name>
```

## Rename local branches

```bash
# This will rename currently checked out branch
git branch -m <new_name>

# Will rename the specified branch
git branch -m <old_branch_name> <new_branch_name>
```

## Rename remote branches

```bash
# delete remote branch
git push origin --delete <old_branch_name>

# push branch with new name
git push -u origin <new_branch_name>
```

## Publishing a local branch to remote

```bash
git push -u origin <local_branch_name>
```

## Setting up tracking between local and remote branches

```bash
# creates a local branch with the same name as the remote branch
git checkout --track origin/<remote_branch_name>

# creates a local branch with the name specified
git branch --track <local_branch_name> origin/<remote_branch_name>
```

## Pulling and pushing branches

```bash
git pull
git push
```

## Comparing commits difference between local and remote branches

```bash
git branch -v
```

## Deleting branches

```bash
# deleting local branch. HEAD has to be pointing to a different branch
git branch -d <local_branch_name>

# deleting remote branch
git push origin --delete <remote_branch_name>
```

## Merging branches

Merging branches preserves all the local history of the branches being merged and creates a new commit with all prior history.

```bash
git switch <branch_to_merge_into>
git merge <branch_containing_commits_to_be_merged>
```

## Rebasing branches

Rebasing merges all the commits into a single linear history and individual branching history is lost

```bash
git switch <branch_to_merge_into>
git rebase <branch_containing_commits_to_be_merged>
```

## Comparing commits between two branches

```bash
# displays commits in branch_B not in branch_A
git log <branch_A>..<branch_B>
```
