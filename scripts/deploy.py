"""Deploy a validated pair of immutable images to Dokploy and verify their revision."""

import json
import os
import re
import time
import urllib.error
import urllib.parse
import urllib.request


def required(name):
    value = os.environ.get(name, "").strip()
    if not value:
        raise RuntimeError(f"Missing {name}")
    return value


def request_json(url, *, body=None, key=None, allow_empty=False):
    headers = {"Accept": "application/json", "Cache-Control": "no-cache"}
    if key:
        headers["x-api-key"] = key
    if body is not None:
        headers["Content-Type"] = "application/json"
    request = urllib.request.Request(
        url,
        data=json.dumps(body).encode() if body is not None else None,
        headers=headers,
    )
    # Never include response bodies or credential-bearing configuration in errors.
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            payload = response.read()
            if allow_empty and not payload.strip():
                return None
            return json.loads(payload)
    except urllib.error.HTTPError as error:
        raise RuntimeError(f"HTTP {error.code} from {urllib.parse.urlsplit(url).path}") from None


class Dokploy:
    def __init__(self, url, key):
        if urllib.parse.urlsplit(url).scheme != "https":
            raise RuntimeError("Dokploy requires HTTPS")
        self.url = url.rstrip("/")
        self.key = key

    def api(self, endpoint, body=None, **query):
        url = f"{self.url}/api/{endpoint}"
        if query:
            url += "?" + urllib.parse.urlencode(query)
        return request_json(url, body=body, key=self.key, allow_empty=body is not None)

    def image(self, application_id, image):
        return self.api(
            "application.saveDockerProvider",
            {
                "applicationId": application_id,
                "dockerImage": image,
                "username": None,
                "password": None,
                "registryUrl": "ghcr.io",
            },
        )

    def deploy(self, application_id, image, title):
        existing = {
            item["deploymentId"]
            for item in self.api("deployment.all", applicationId=application_id)
        }
        self.image(application_id, image)
        self.api("application.deploy", {"applicationId": application_id, "title": title})
        deadline = time.monotonic() + 600
        while time.monotonic() < deadline:
            deployments = self.api("deployment.all", applicationId=application_id)
            current = [
                item
                for item in deployments
                if item["deploymentId"] not in existing and item.get("title") == title
            ]
            if current:
                status = current[0].get("status")
                if status == "done":
                    return
                if status in {"error", "cancelled", "canceled"}:
                    raise RuntimeError(f"Dokploy deployment {title} failed: {status}")
            time.sleep(10)
        raise RuntimeError(f"Timed out waiting for deployment {title}")


def verify_health(origin, revision, frontend=False, timeout=240):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        try:
            body = request_json(origin.rstrip("/") + "/api/health")
            ready = body.get("nectar") == "ok" if frontend else body.get("status") == "ok"
            if ready and body.get("revision") == revision:
                if frontend and body.get("hiveway", {}).get("revision") != revision:
                    raise RuntimeError("Frontend is connected to a different backend release")
                if not frontend:
                    ready = request_json(origin.rstrip("/") + "/api/ready")
                    if ready.get("status") != "ok":
                        raise RuntimeError("Backend dependencies are not ready")
                return
        except (RuntimeError, OSError, ValueError):
            pass
        time.sleep(10)
    raise RuntimeError(f"Health or revision verification failed for {origin}")


def immutable_image(image, app):
    if not re.fullmatch(rf"ghcr\.io/[a-z0-9_.-]+/hive-{app}@sha256:[0-9a-f]{{64}}", image):
        raise RuntimeError(f"Expected an immutable GHCR digest for {app}")
    return image


def verify_public_image(image):
    repository, digest = image.removeprefix("ghcr.io/").split("@", 1)
    token_url = "https://ghcr.io/token?" + urllib.parse.urlencode({"service": "ghcr.io", "scope": f"repository:{repository}:pull"})
    try:
        token = request_json(token_url)["token"]
        request = urllib.request.Request(
            f"https://ghcr.io/v2/{repository}/manifests/{digest}",
            method="HEAD",
            headers={"Authorization": f"Bearer {token}", "Accept": "application/vnd.oci.image.index.v1+json, application/vnd.docker.distribution.manifest.v2+json"},
        )
        with urllib.request.urlopen(request, timeout=30):
            pass
    except (RuntimeError, OSError, KeyError):
        raise RuntimeError(f"{repository} must be Public in GitHub package settings before deployment") from None


def main():
    revision = required("RELEASE_REVISION")
    if not re.fullmatch(r"[0-9a-f]{40}", revision):
        raise RuntimeError("Expected a full Git revision")
    client = Dokploy(required("DOKPLOY_URL"), required("DOKPLOY_API_KEY"))
    releases = [
        (app, required(f"DOKPLOY_{app.upper()}_ID"), immutable_image(required(f"{app.upper()}_IMAGE"), app), required(f"{app.upper()}_PUBLIC_URL"))
        for app in ["hiveway", "nectar"]
    ]
    # Both releases must be anonymously pullable before mutating either service.
    for _, _, image, _ in releases:
        verify_public_image(image)
    # Resolve all inputs before mutating either service. Deploy the API first.
    for app, application_id, image, origin in releases:
        print(f"Deploying {app}: {image}", flush=True)
        client.deploy(application_id, image, f"Release {revision} ({app})")
        verify_health(origin, revision, frontend=app == "nectar")
        print(f"Verified {app} at {revision}", flush=True)
    summary = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary:
        with open(summary, "a", encoding="utf-8") as output:
            output.write(f"Deployed and verified release `{revision}` on Hiveway and Nectar.\n")


if __name__ == "__main__":
    main()
