# Remote Editing (Experimental)

Fresh supports editing files on remote machines via SSH using the `user@host:path` syntax. This is useful for editing files on servers without needing to install Fresh remotely.

```bash
# Open a specific file
fresh deploy@server.example.com:/etc/nginx/nginx.conf

# Open home directory in file explorer
fresh user@host:~

# Open with line number
fresh user@host:/var/log/app.log:100
```

**Features:**
- Password and SSH key authentication
- File explorer shows remote directory
- Sudo save support for protected files
- Status bar shows `[SSH:user@host]` indicator

**Requirements:**
- SSH access to the remote host
- Python 3 installed on the remote host (for the agent)

## Alternative: SSH + Session Persistence

If you need a persistent editing session that survives connection drops, consider running Fresh directly on the remote host with [Session Persistence](./session-persistence.md):

```bash
ssh user@host
fresh -a        # start a persistent session on the remote host
# if SSH disconnects, just reconnect and reattach:
ssh user@host
fresh -a
```

You can also pair SSH with `tmux` for a similar effect—run `tmux` on the remote host and launch Fresh inside it. Session persistence has the advantage of being built into Fresh, so editor state (open files, terminals, undo history) is preserved without an external multiplexer.
