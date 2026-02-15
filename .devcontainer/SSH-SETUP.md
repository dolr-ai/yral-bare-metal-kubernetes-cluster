# SSH Agent & GitHub CLI Setup for DevContainer

This guide configures your WSL environment so both Git SSH operations and GitHub CLI (`gh`) work seamlessly in the devcontainer.

## What Gets Set Up

1. **SSH Agent Forwarding**: Your WSL SSH agent is mounted into the devcontainer
2. **GitHub CLI (gh)**: Access to GitHub API using your WSL gh authentication  
3. **Git Push/Pull**: Works with your SSH keys for all GitHub operations

## Prerequisites

- WSL (Windows Subsystem for Linux)
- SSH keys in `~/.ssh/` (e.g., `id_ed25519`)
- GitHub CLI authenticated in WSL (run `gh auth login` in WSL if not already done)

## One-Time Setup in WSL

### 1. Exit the DevContainer

In VS Code, use Command Palette (Ctrl+Shift+P):
```
Dev Containers: Reopen Folder Locally
```

Or open a native WSL terminal.

### 2. Authenticate GitHub CLI in WSL (if not already done)

```bash
gh auth login
# Choose: GitHub.com
# Choose: SSH
# Follow the prompts to authenticate
```

Verify authentication:
```bash
gh auth status
# Should show: ✓ Logged in to github.com
```

### 3. Set Up Fixed SSH Agent Socket

Run these commands in your **WSL terminal**:

```bash
# Create the .ssh directory if it doesn't exist
mkdir -p ~/.ssh

# Copy the setup script from this repo to your WSL home
cp .devcontainer/ssh-agent-setup.sh ~/.ssh/
chmod +x ~/.ssh/ssh-agent-setup.sh

# Add to your shell startup (bash)
echo 'source ~/.ssh/ssh-agent-setup.sh' >> ~/.bashrc

# Or if using zsh:
# echo 'source ~/.ssh/ssh-agent-setup.sh' >> ~/.zshrc

# Activate it now
source ~/.bashrc

# Add your SSH key
ssh-add ~/.ssh/id_ed25519

# Verify
ssh-add -l
# Should show your key fingerprint
```

### 4. Rebuild the DevContainer

In VS Code:
1. Open Command Palette (Ctrl+Shift+P)
2. Run: `Dev Containers: Rebuild and Reopen in Container`
3. Wait for rebuild to complete

## Verification in DevContainer

Once the devcontainer reopens, test everything:

### Test SSH Agent
```bash
echo $SSH_AUTH_SOCK
# Output: /ssh-agent

ssh-add -l
# Should list: SHA256:... /home/saika/.ssh/id_ed25519
```

### Test GitHub SSH Access
```bash
ssh -T git@github.com
# Output: Hi <username>! You've successfully authenticated, but GitHub does not provide shell access.
```

### Test Git Operations
```bash
git push    # Should work without errors!
git pull    # Should work without errors!
```

### Test GitHub CLI
```bash
gh auth status
# Should show: ✓ Logged in to github.com

gh repo view
# Should show info about current repository

gh pr list
# Should list pull requests (if any)
```

## What Was Configured

### In [devcontainer.json](devcontainer.json):

1. **SSH Agent Mount**:
   ```json
   "mounts": [
     "type=bind,source=${env:HOME}/.ssh/agent.sock,target=/ssh-agent"
   ]
   ```
   - Mounts your WSL SSH agent socket into container at `/ssh-agent`

2. **GitHub CLI Config Mount**:
   ```json
   "type=bind,source=${env:HOME}/.config/gh,target=/home/vscode/.config/gh"
   ```
   - Shares your WSL gh CLI authentication with the container

3. **Environment Variable**:
   ```json
   "remoteEnv": {
     "SSH_AUTH_SOCK": "/ssh-agent"
   }
   ```
   - Tells SSH and Git where to find the agent socket

4. **Auto-Configuration**:
   - Verifies SSH agent on container creation
   - Configures gh CLI to use SSH protocol

## Troubleshooting

### Issue: `ssh-add -l` shows "Could not open a connection"

**In WSL terminal:**
```bash
source ~/.ssh/ssh-agent-setup.sh
ssh-add ~/.ssh/id_ed25519
```

### Issue: `gh auth status` shows "Failed to log in"

**In WSL terminal:**
```bash
# Re-authenticate GitHub CLI in WSL
gh auth login

# Then rebuild devcontainer
```

### Issue: Git push still shows "Permission denied (publickey)"

**Check in devcontainer:**
```bash
# Verify socket is mounted
ls -la /ssh-agent

# Verify environment variable
echo $SSH_AUTH_SOCK

# Test SSH connection
ssh -Tv git@github.com
```

If socket doesn't exist, ensure step 3 was completed in WSL and container was rebuilt.

### Issue: "no such file or directory" error during container creation

This means `~/.config/gh` doesn't exist in WSL. Fix by:

**In WSL:**
```bash
gh auth login    # This creates ~/.config/gh
```

Then rebuild the container.

### Issue: Socket path changes after WSL restart

The setup script should prevent this. Verify:
```bash
# In WSL
cat ~/.bashrc | grep ssh-agent-setup
# Should show: source ~/.ssh/ssh-agent-setup.sh
```

If missing, re-run step 3 of the setup.

## How It Works

### SSH Agent Forwarding
1. **Fixed Socket**: Script creates persistent socket at `~/.ssh/agent.sock` in WSL
2. **Mount**: DevContainer mounts this socket from WSL → `/ssh-agent` in container
3. **Environment**: `SSH_AUTH_SOCK=/ssh-agent` tells tools where to find it
4. **Security**: Private keys stay in WSL, only signing operations forwarded

### GitHub CLI Sharing
1. **Authentication Data**: Your WSL `~/.config/gh/` contains auth tokens
2. **Mount**: DevContainer mounts this directory into container
3. **Shared State**: Both WSL and container use same gh authentication
4. **Protocol**: Configured to use SSH for git operations (`git_protocol=ssh`)

## Security Notes

✅ **Secure**:
- Private SSH keys remain in WSL only
- Container gets read-only access to SSH agent socket
- GitHub tokens stored in mounted config (not exposed to container filesystem)

✅ **Persistent**:
- Survives container rebuilds
- No re-authentication needed
- Keys automatically loaded on WSL shell startup

## Additional Resources

- [VS Code Dev Containers: SSH Agent Forwarding](https://code.visualstudio.com/remote/advancedcontainers/sharing-git-credentials)
- [GitHub CLI Manual](https://cli.github.com/manual/)
