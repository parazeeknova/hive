"""Apply a pinned public GitHub starter locally. Requires HIVE_API_KEY; never overwrites a target."""

import argparse
import hashlib
import gzip
import io
import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess
import tarfile
import urllib.parse
import urllib.request


MAX_ARCHIVE = 8 * 1024 * 1024
MAX_FILES = 2000
MAX_CONTENT = 32 * 1024 * 1024


def fetch(url, key=None, body=None):
    request = urllib.request.Request(url, data=json.dumps(body).encode() if body is not None else None,
        headers={**({"Authorization": f"Bearer {key}"} if key else {}), "User-Agent": "Hive-scaffold/1.0", "Content-Type": "application/json", "Accept": "application/json, application/gzip"})
    # Do not forward the bearer credential across redirects.
    class NoRedirect(urllib.request.HTTPRedirectHandler):
        def redirect_request(self, req, fp, code, msg, headers, newurl):
            return None
    with urllib.request.build_opener(NoRedirect).open(request, timeout=30) as response:
        data = response.read(MAX_ARCHIVE + 1)
        if len(data) > MAX_ARCHIVE:
            raise ValueError("Scaffold response exceeds size limit")
        return data


def verify_bundle(plan, archive):
    if len(archive) > MAX_ARCHIVE:
        raise ValueError("Template archive exceeds download limit")
    if hashlib.sha256(archive).hexdigest() != plan["archive_sha256"]:
        raise ValueError("Template archive checksum mismatch")
    files = {}
    total = 0
    archive_root = None
    with gzip.GzipFile(fileobj=io.BytesIO(archive)) as compressed:
        expanded = compressed.read(MAX_CONTENT + 1)
    if len(expanded) > MAX_CONTENT:
        raise ValueError("Template expanded archive exceeds limit")
    with tarfile.open(fileobj=io.BytesIO(expanded), mode="r:") as tar:
        for index, item in enumerate(tar):
            if index >= MAX_FILES * 2:
                raise ValueError("Too many archive entries")
            path = PurePosixPath(item.name)
            if path.is_absolute() or ".." in path.parts or "\\" in item.name or any(":" in part or part.rstrip(" .").lower() == ".git" for part in path.parts):
                raise ValueError("Unsafe template path")
            if not path.parts:
                raise ValueError("Empty template path")
            root, *parts = path.parts
            archive_root = archive_root or root
            if root != archive_root:
                raise ValueError("Archive must have a single source root")
            if not item.isdir() and not item.isfile():
                raise ValueError("Links and special files are not supported")
            if item.isdir():
                continue
            if not parts:
                raise ValueError("Missing source root")
            name = str(PurePosixPath(*parts))
            if name in files or len(files) >= MAX_FILES:
                raise ValueError("Duplicate path or too many files")
            total += item.size
            if total > MAX_CONTENT:
                raise ValueError("Template expanded content exceeds limit")
            content = tar.extractfile(item).read(item.size + 1)
            if len(content) != item.size:
                raise ValueError("Truncated template file")
            files[name] = (content, bool(item.mode & 0o111))
    if not files:
        raise ValueError("Template contains no files")
    return files


def apply_bundle(plan, archive, target):
    files = verify_bundle(plan, archive)
    target = Path(target)
    target.mkdir(mode=0o755, parents=False, exist_ok=False)
    for name, (content, executable) in files.items():
        destination = target / name
        destination.parent.mkdir(parents=True, exist_ok=True)
        with destination.open("xb") as output:
            output.write(content)
        destination.chmod(0o755 if executable else 0o644)
    return len(files)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("repository_url", help="Public https://github.com/owner/repository URL")
    parser.add_argument("revision", help="Full 40-character commit SHA")
    parser.add_argument("directory", type=Path)
    args = parser.parse_args()
    source = re.fullmatch(r"https://github\.com/([A-Za-z0-9_.-]+)/([A-Za-z0-9_.-]+)/?", args.repository_url)
    if not source or any(part in {".", ".."} for part in source.groups()) or not re.fullmatch(r"[a-fA-F0-9]{40}", args.revision):
        parser.error("Provide a public GitHub repository URL and full commit SHA")
    name = args.directory.name
    if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_-]{0,62}", name):
        parser.error("Use a directory name with letters, digits, hyphens or underscores")
    if args.directory.exists() or args.directory.is_symlink():
        parser.error("Target already exists; choose a new directory")
    key = os.environ.get("HIVE_API_KEY", "").strip()
    if not key:
        parser.error("Set HIVE_API_KEY to your Hive API key")
    origin = os.environ.get("HIVE_ORIGIN", "https://hiveway.itssingularity.com").rstrip("/")
    parsed = urllib.parse.urlsplit(origin)
    if parsed.scheme != "https" or parsed.path or parsed.query or parsed.fragment or parsed.username or parsed.password:
        parser.error("HIVE_ORIGIN must be an HTTPS origin")
    response = json.loads(fetch(origin + "/mcp", key, {"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "scaffold_project", "arguments": {"project_name": name, "repository_url": args.repository_url, "revision": args.revision}}}))
    if response.get("error") or response.get("result", {}).get("isError"):
        raise RuntimeError("Hive rejected the scaffold request")
    plan = response["result"]["structuredContent"]
    archive_url = f"https://codeload.github.com/{source[1]}/{source[2]}/tar.gz/{args.revision}"
    if plan["archive_url"] != archive_url or plan["project_name"] != name:
        raise ValueError("Unexpected template origin or project name")
    archive = fetch(archive_url)
    count = apply_bundle(plan, archive, args.directory)
    subprocess.run(["git", "init", str(args.directory)], check=True)
    print(f"Created {args.directory}: {count} files from {plan['revision']}")
    print("Original workspace/package names are preserved. Read the source README for its prerequisites and setup commands. No dependency installation or template scripts were executed.")


if __name__ == "__main__":
    main()
