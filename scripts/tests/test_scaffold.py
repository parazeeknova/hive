import hashlib
import importlib.util
import io
from pathlib import Path
import tarfile
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("scaffold", Path(__file__).parents[1] / "scaffold.py")
scaffold = importlib.util.module_from_spec(spec)
spec.loader.exec_module(scaffold)


def bundle(entries):
    buffer = io.BytesIO()
    with tarfile.open(fileobj=buffer, mode="w:gz") as tar:
        for name, content, kind in entries:
            member = tarfile.TarInfo(name)
            member.type = kind
            member.mode = 0o755 if name.endswith(".sh") else 0o644
            member.linkname = "/etc/passwd" if kind == tarfile.SYMTYPE else ""
            member.size = len(content)
            tar.addfile(member, io.BytesIO(content))
    archive = buffer.getvalue()
    return {"archive_sha256": hashlib.sha256(archive).hexdigest()}, archive


class ScaffoldTests(unittest.TestCase):
    def setUp(self):
        self.plan, self.archive = bundle([("starter-abc/README.md", b"hello", tarfile.REGTYPE), ("starter-abc/.env.example", b"PUBLIC=example", tarfile.REGTYPE), ("starter-abc/run.sh", b"echo hello", tarfile.REGTYPE)])

    def test_arbitrary_bundle_preserves_files_modes_and_refuses_existing_target(self):
        with tempfile.TemporaryDirectory() as parent:
            target = Path(parent) / "new-app"
            self.assertEqual(scaffold.apply_bundle(self.plan, self.archive, target), 3)
            self.assertEqual((target / "README.md").read_text(), "hello")
            self.assertTrue((target / ".env.example").exists())
            self.assertTrue((target / "run.sh").stat().st_mode & 0o111)
            self.assertFalse((target / "starter-abc").exists())
            with self.assertRaises(FileExistsError):
                scaffold.apply_bundle(self.plan, self.archive, target)

    def test_corrupt_bundle_is_rejected_before_creating_directory(self):
        with tempfile.TemporaryDirectory() as parent:
            target = Path(parent) / "app"
            with self.assertRaises(ValueError):
                scaffold.apply_bundle(self.plan, self.archive + b"bad", target)
            self.assertFalse(target.exists())

    def test_unsafe_archives_are_rejected(self):
        cases = [
            [("../escape", b"", tarfile.REGTYPE)],
            [("root/../escape", b"", tarfile.REGTYPE)],
            [("/root/file", b"", tarfile.REGTYPE)],
            [("root/link", b"", tarfile.SYMTYPE)],
            [("root/.git/config", b"", tarfile.REGTYPE)],
            [("root/a", b"", tarfile.REGTYPE), ("other/b", b"", tarfile.REGTYPE)],
            [("root/a", b"", tarfile.REGTYPE), ("root/a", b"", tarfile.REGTYPE)],
        ]
        for entries in cases:
            plan, archive = bundle(entries)
            with self.subTest(entries=entries), self.assertRaises(ValueError):
                scaffold.verify_bundle(plan, archive)

    def test_oversized_expansion_is_rejected(self):
        plan, archive = bundle([("root/big", b"x" * 50, tarfile.REGTYPE)])
        from unittest.mock import patch
        with patch.object(scaffold, "MAX_CONTENT", 10), self.assertRaises(ValueError):
            scaffold.verify_bundle(plan, archive)


if __name__ == "__main__":
    unittest.main()
