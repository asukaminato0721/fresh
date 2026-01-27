#!/usr/bin/env python3
"""Fresh Remote Agent - bootstrapped via SSH stdin"""
import sys
import os
import json
import base64
import stat
import shutil
import subprocess
import threading
import select

CHUNK = 65536
VERSION = 1

# Active background processes: id -> Popen
procs = {}
# Request IDs marked for cancellation
cancelled = set()
# Lock for thread-safe access
lock = threading.Lock()


def send(id, **kw):
    """Send a JSON message to stdout."""
    msg = {"id": id, **kw}
    sys.stdout.write(json.dumps(msg, separators=(",", ":")) + "\n")
    sys.stdout.flush()


def b64(data):
    """Encode bytes to base64 string."""
    return base64.b64encode(data).decode("ascii")


def unb64(s):
    """Decode base64 string to bytes."""
    return base64.b64decode(s)


def validate_path(p):
    """Validate and canonicalize a path."""
    if not p:
        raise ValueError("empty path")
    expanded = os.path.expanduser(p)
    if not os.path.isabs(expanded):
        expanded = os.path.abspath(expanded)
    return os.path.realpath(expanded)


# === File Operations ===


def cmd_read(id, p):
    """Read file contents, streaming in chunks for large files."""
    path = validate_path(p["path"])
    off = p.get("off", 0)
    length = p.get("len")

    with open(path, "rb") as f:
        if off:
            f.seek(off)
        total = 0
        while True:
            to_read = min(CHUNK, length - total) if length else CHUNK
            chunk = f.read(to_read)
            if not chunk:
                break
            total += len(chunk)
            send(id, d={"data": b64(chunk)})
            if length and total >= length:
                break
    send(id, r={"size": total})


def cmd_write(id, p):
    """Write file contents atomically."""
    path = validate_path(p["path"])
    data = unb64(p["data"])

    # Atomic write: write to temp, then rename
    tmp = f"{path}.fresh-{os.getpid()}"
    try:
        # Preserve permissions if file exists
        mode = None
        if os.path.exists(path):
            mode = os.stat(path).st_mode

        with open(tmp, "wb") as f:
            f.write(data)
            f.flush()
            os.fsync(f.fileno())

        if mode is not None:
            os.chmod(tmp, mode)

        os.rename(tmp, path)
    finally:
        if os.path.exists(tmp):
            try:
                os.unlink(tmp)
            except OSError:
                pass

    send(id, r={"size": len(data)})


def cmd_stat(id, p):
    """Get file/directory metadata."""
    path = validate_path(p["path"])
    follow = p.get("link", True)

    st = os.stat(path, follow_symlinks=follow)
    is_link = stat.S_ISLNK(os.lstat(path).st_mode) if follow else False

    send(
        id,
        r={
            "size": st.st_size,
            "mtime": int(st.st_mtime),
            "mode": st.st_mode,
            "uid": st.st_uid,
            "gid": st.st_gid,
            "dir": stat.S_ISDIR(st.st_mode),
            "file": stat.S_ISREG(st.st_mode),
            "link": is_link,
        },
    )


def cmd_ls(id, p):
    """List directory contents with metadata."""
    path = validate_path(p["path"])
    entries = []

    for entry in os.scandir(path):
        try:
            st = entry.stat(follow_symlinks=False)
            is_link = entry.is_symlink()

            # For symlinks, check target type
            target_is_dir = False
            if is_link:
                try:
                    target_is_dir = entry.is_dir(follow_symlinks=True)
                except OSError:
                    pass

            entries.append(
                {
                    "name": entry.name,
                    "path": os.path.join(path, entry.name),
                    "dir": entry.is_dir(follow_symlinks=True),
                    "file": entry.is_file(follow_symlinks=True),
                    "link": is_link,
                    "link_dir": target_is_dir,
                    "size": st.st_size,
                    "mtime": int(st.st_mtime),
                    "mode": st.st_mode,
                }
            )
        except OSError:
            # Skip entries we can't stat
            pass

    send(id, r={"entries": entries})


def cmd_rm(id, p):
    """Remove a file."""
    os.unlink(validate_path(p["path"]))
    send(id, r={})


def cmd_rmdir(id, p):
    """Remove an empty directory."""
    os.rmdir(validate_path(p["path"]))
    send(id, r={})


def cmd_mkdir(id, p):
    """Create a directory."""
    path = validate_path(p["path"])
    if p.get("parents"):
        os.makedirs(path, exist_ok=True)
    else:
        os.mkdir(path)
    send(id, r={})


def cmd_mv(id, p):
    """Move/rename a file or directory.

    Uses shutil.move() to handle cross-device moves (e.g., /tmp to /etc).
    """
    shutil.move(validate_path(p["from"]), validate_path(p["to"]))
    send(id, r={})


def cmd_cp(id, p):
    """Copy a file."""
    dst = validate_path(p["to"])
    shutil.copy2(validate_path(p["from"]), dst)
    send(id, r={"size": os.path.getsize(dst)})


def cmd_realpath(id, p):
    """Get canonical absolute path."""
    send(id, r={"path": validate_path(p["path"])})


def cmd_chmod(id, p):
    """Change file permissions."""
    os.chmod(validate_path(p["path"]), p["mode"])
    send(id, r={})


def cmd_exists(id, p):
    """Check if path exists."""
    try:
        path = validate_path(p["path"])
        send(id, r={"exists": os.path.exists(path)})
    except (ValueError, OSError):
        send(id, r={"exists": False})


# === Process Operations ===


def cmd_exec(id, p):
    """Execute a process with streaming output."""
    cwd = validate_path(p["cwd"]) if p.get("cwd") else None
    cmd = p["cmd"]
    args = p.get("args", [])

    try:
        proc = subprocess.Popen(
            [cmd] + args,
            cwd=cwd,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
    except FileNotFoundError:
        send(id, e=f"command not found: {cmd}")
        return
    except PermissionError:
        send(id, e=f"permission denied: {cmd}")
        return

    with lock:
        procs[id] = proc

    def stream_output():
        """Stream process output in a background thread."""
        try:
            while proc.poll() is None:
                # Check for cancellation
                if id in cancelled:
                    proc.terminate()
                    try:
                        proc.wait(timeout=2)
                    except subprocess.TimeoutExpired:
                        proc.kill()
                    send(id, e="cancelled")
                    return

                # Non-blocking read from stdout and stderr
                readable, _, _ = select.select(
                    [proc.stdout, proc.stderr], [], [], 0.05
                )
                for fd in readable:
                    data = fd.read(4096)
                    if data:
                        key = "out" if fd == proc.stdout else "err"
                        send(id, d={key: b64(data)})

            # Drain any remaining output
            out, err = proc.communicate(timeout=5)
            if out:
                send(id, d={"out": b64(out)})
            if err:
                send(id, d={"err": b64(err)})

            send(id, r={"code": proc.returncode})
        except Exception as e:
            send(id, e=str(e))
        finally:
            with lock:
                procs.pop(id, None)
                cancelled.discard(id)

    threading.Thread(target=stream_output, daemon=True).start()


def cmd_kill(id, p):
    """Kill a background process."""
    target_id = p["id"]
    with lock:
        proc = procs.get(target_id)

    if proc:
        proc.terminate()
        try:
            proc.wait(timeout=2)
        except subprocess.TimeoutExpired:
            proc.kill()
        send(id, r={})
    else:
        send(id, e="process not found")


def cmd_cancel(id, p):
    """Cancel an in-flight request."""
    target_id = p["id"]
    cancelled.add(target_id)

    with lock:
        proc = procs.get(target_id)
    if proc:
        proc.terminate()

    send(id, r={})


# === Method dispatch ===

METHODS = {
    "read": cmd_read,
    "write": cmd_write,
    "stat": cmd_stat,
    "ls": cmd_ls,
    "rm": cmd_rm,
    "rmdir": cmd_rmdir,
    "mkdir": cmd_mkdir,
    "mv": cmd_mv,
    "cp": cmd_cp,
    "realpath": cmd_realpath,
    "chmod": cmd_chmod,
    "exists": cmd_exists,
    "exec": cmd_exec,
    "kill": cmd_kill,
    "cancel": cmd_cancel,
}


def handle_request(line):
    """Parse and handle a single request."""
    try:
        req = json.loads(line)
    except json.JSONDecodeError as e:
        send(0, e=f"parse error: {e}")
        return

    id = req.get("id", 0)
    method = req.get("m")
    params = req.get("p", {})

    if method not in METHODS:
        send(id, e=f"unknown method: {method}")
        return

    try:
        METHODS[method](id, params)
    except PermissionError as e:
        send(id, e=f"permission denied: {e}")
    except FileNotFoundError as e:
        send(id, e=f"not found: {e}")
    except IsADirectoryError as e:
        send(id, e=f"is a directory: {e}")
    except NotADirectoryError as e:
        send(id, e=f"not a directory: {e}")
    except OSError as e:
        send(id, e=f"os error: {e}")
    except Exception as e:
        send(id, e=str(e))


def main():
    """Main entry point."""
    # Send ready message
    send(0, ok=True, v=VERSION)

    # Process requests from stdin
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        handle_request(line)


if __name__ == "__main__":
    main()
