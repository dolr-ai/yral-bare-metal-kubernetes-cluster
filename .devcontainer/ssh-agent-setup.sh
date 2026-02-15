#!/bin/bash
# SSH Agent Setup Script for WSL
# This script ensures SSH agent runs with a fixed socket path
# Place this in ~/.ssh/ssh-agent-setup.sh and source it from your shell RC file

export SSH_AUTH_SOCK="$HOME/.ssh/agent.sock"

# Check if agent socket exists and is valid
if [ -S "$SSH_AUTH_SOCK" ]; then
    # Test if agent is responsive
    ssh-add -l >/dev/null 2>&1
    agent_status=$?
    
    if [ $agent_status -eq 0 ] || [ $agent_status -eq 1 ]; then
        # Agent is running (either has identities or is empty)
        return 0
    fi
fi

# Agent not running or not responsive - start new one
echo "Starting SSH agent with fixed socket at $SSH_AUTH_SOCK"
rm -f "$SSH_AUTH_SOCK"
eval "$(ssh-agent -a "$SSH_AUTH_SOCK")" > /dev/null

# Automatically add default keys if they exist and aren't already added
if [ -f "$HOME/.ssh/id_ed25519" ] || [ -f "$HOME/.ssh/id_rsa" ]; then
    ssh-add -l >/dev/null 2>&1
    if [ $? -eq 1 ]; then
        # Agent is running but has no identities - add keys
        echo "Adding SSH keys to agent..."
        [ -f "$HOME/.ssh/id_ed25519" ] && ssh-add "$HOME/.ssh/id_ed25519" 2>/dev/null
        [ -f "$HOME/.ssh/id_rsa" ] && ssh-add "$HOME/.ssh/id_rsa" 2>/dev/null
    fi
fi
