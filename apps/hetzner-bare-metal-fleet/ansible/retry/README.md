# Ansible Retry Files Directory

This directory stores Ansible retry files that are automatically generated when playbook runs encounter failures.

## What are Retry Files?

When a playbook fails on one or more hosts, Ansible creates a `.retry` file containing the list of failed hosts. This allows you to easily re-run the playbook targeting only those hosts that failed.

## Format

Retry files contain one hostname per line:
```
offchain-agent-1
vault-2
```

## Usage

To retry failed hosts:
```bash
ansible-playbook ansible/playbooks/system-update.yml --limit @ansible/retry/system-update.retry
```

## Note

These files are automatically ignored by git (see `.gitignore`) as they are temporary and specific to each run.
