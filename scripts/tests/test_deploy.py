"""Regression checks for release identity and Dokploy queue handling."""

import importlib.util
from pathlib import Path
import unittest
from unittest.mock import Mock, patch

spec = importlib.util.spec_from_file_location("deploy", Path(__file__).parents[1] / "deploy.py")
deploy = importlib.util.module_from_spec(spec)
spec.loader.exec_module(deploy)


class DeploymentTests(unittest.TestCase):
    def test_only_matching_immutable_images_are_accepted(self):
        image = "ghcr.io/parazeeknova/hive-nectar@sha256:" + "a" * 64
        self.assertEqual(deploy.immutable_image(image, "nectar"), image)
        for invalid in [image.replace("nectar", "hiveway"), "ghcr.io/parazeeknova/hive-nectar:main", image + "x"]:
            with self.assertRaises(RuntimeError):
                deploy.immutable_image(invalid, "nectar")

    def test_old_success_does_not_complete_new_deployment(self):
        client = deploy.Dokploy("https://dokploy.example", "private")
        old = {"deploymentId": "old", "title": "release", "status": "done"}
        new = {"deploymentId": "new", "title": "release", "status": "done"}
        client.api = Mock(side_effect=[[old], None, None, [old], [new, old]])
        with patch.object(deploy.time, "sleep") as sleep:
            client.deploy("app", "image", "release")
        sleep.assert_called_once_with(10)

    def test_deployment_failure_is_reported(self):
        client = deploy.Dokploy("https://dokploy.example", "private")
        client.api = Mock(side_effect=[[], None, None, [{"deploymentId": "new", "title": "release", "status": "error"}]])
        with self.assertRaisesRegex(RuntimeError, "failed"):
            client.deploy("app", "image", "release")

    def test_frontend_must_reach_backend_at_same_revision(self):
        wrong = {"nectar": "ok", "revision": "new", "hiveway": {"revision": "old"}}
        right = {"nectar": "ok", "revision": "new", "hiveway": {"revision": "new"}}
        with patch.object(deploy, "request_json", side_effect=[wrong, right]), patch.object(deploy.time, "sleep") as sleep:
            deploy.verify_health("https://hive.example", "new", frontend=True)
        sleep.assert_called_once_with(10)

    def test_backend_requires_database_and_cache_readiness(self):
        with patch.object(deploy, "request_json", side_effect=[{"status": "ok", "revision": "new"}, {"status": "ok"}]) as request:
            deploy.verify_health("https://api.example", "new")
        self.assertEqual(request.call_args.args[0], "https://api.example/api/ready")


if __name__ == "__main__":
    unittest.main()
