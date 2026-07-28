# Ansible Vault Setup Guide

Secrets are managed with [Ansible Vault](https://docs.ansible.com/ansible/latest/vault_guide/index.html). The encrypted `vars/vault.yml` is committed to the repository; the vault password is stored only in `.vault_pass` (gitignored locally) and as the `ANSIBLE_VAULT_PASSWORD` secret in GitHub Actions.

## Quick Start

```bash
cd ansible
./init_vault.sh
```

Follow the prompts to set a vault password and enter your Cloudflare credentials. The script creates `.vault_pass`, encrypts `vars/vault.yml`, and generates the `.env` file.

## Directory Structure

```
ansible/
├── ansible.cfg              # Vault password file path, inventory location
├── .vault_pass              # Vault password (gitignored — never commit)
├── inventory/
│   └── hosts                # Local inventory (localhost)
├── vars/
│   ├── main.yml             # Plaintext variable references (committed)
│   ├── main.yml.example     # Template for reviewing variable names
│   ├── vault.yml            # Ansible-vault-encrypted secrets (committed)
│   └── vault.yml.example    # Template showing expected vault keys
├── templates/
│   └── env.j2               # Template for .env file
├── setup_env.yml            # Playbook: decrypt vault → write .env
└── init_vault.sh            # Interactive first-time setup script
```

## Variable Naming Convention

- **`vars/main.yml`** (committed) — references the vault for sensitive values; stores public identifiers in plaintext:
  ```yaml
  cloudflare_api_token: "{{ vault_cloudflare_api_token }}"  # sensitive → vault
  cloudflare_zone_id: "abc123xyz"     # public identifier — plaintext is fine
  cloudflare_account_id: "def456uvw"  # public identifier — plaintext is fine
  ```

- **`vars/vault.yml`** (encrypted, committed) — only truly secret values:
  ```yaml
  vault_cloudflare_api_token: "your_actual_token"
  ```

## Common Commands

```bash
# View encrypted secrets
ansible-vault view vars/vault.yml

# Edit secrets (decrypts, opens $EDITOR, re-encrypts on save)
ansible-vault edit vars/vault.yml

# Regenerate .env after editing secrets
ansible-playbook setup_env.yml

# Change vault password
ansible-vault rekey vars/vault.yml
```

## Adding a New Secret

1. **Edit `vars/main.yml`** — if the value is sensitive, add a vault reference; if it's a public identifier, add it in plaintext:
   ```yaml
   # Sensitive:
   new_api_key: "{{ vault_new_api_key }}"
   # Public identifier:
   service_project_id: "proj-abc123"
   ```

2. **For sensitive values only — edit the vault:**
   ```bash
   ansible-vault edit vars/vault.yml
   ```
   Add:
   ```yaml
   vault_new_api_key: "your_secret"
   ```

3. **Update `templates/env.j2`** — expose it as an env var:
   ```
   NEW_API_KEY={{ new_api_key }}
   ```

4. **Regenerate `.env`**:
   ```bash
   ansible-playbook setup_env.yml
   ```

## CI/CD

GitHub Actions reads the vault password from the `ANSIBLE_VAULT_PASSWORD` repository secret, writes it to `ansible/.vault_pass`, and runs `ansible-playbook ansible/setup_env.yml` to generate `.env` before any steps that need credentials.

To add the secret: **GitHub repo → Settings → Secrets and variables → Actions → New repository secret → `ANSIBLE_VAULT_PASSWORD`**.

## Security

- **Never commit** `.vault_pass` or `.env`
- **Always commit** the encrypted `vars/vault.yml`
- Share the vault password via a password manager (not Slack/email)
