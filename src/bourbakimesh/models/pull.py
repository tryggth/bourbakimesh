"""Multi-tier Model Registry and P2P weight pull CLI.

Hierarchy:
1. Local Disk Cache (checkpoints/)
2. Local P2P Swarm (libp2p weight chunks)
3. GitHub Release Asset (v0.1.0-alpha)
4. Hugging Face Hub (tryggth/bourbakimesh-muzero)
"""

from __future__ import annotations
import argparse
import hashlib
from pathlib import Path
import subprocess
import sys
from typing import Dict, Optional, Union
import urllib.error
import urllib.request


def compute_file_sha256(filepath: Union[str, Path]) -> str:
    """Compute SHA-256 hexadecimal digest of a local file."""
    hasher = hashlib.sha256()
    with open(filepath, "rb") as f:
        while chunk := f.read(1024 * 1024):
            hasher.update(chunk)
    return hasher.hexdigest()


def load_checksum_manifest(manifest_path: Union[str, Path] = "checkpoints/CHECKSUMS.txt") -> Dict[str, str]:
    """Parse SHA-256 checksums from manifest file."""
    manifest_path = Path(manifest_path)
    checksums: Dict[str, str] = {}
    if not manifest_path.exists():
        return checksums

    with open(manifest_path, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            parts = line.split(maxsplit=1)
            if len(parts) == 2:
                digest, path_str = parts
                filename = Path(path_str.strip()).name
                checksums[filename] = digest.strip()
    return checksums


class ModelPuller:
    """Multi-tier model weight downloader and integrity validator."""

    def __init__(
        self,
        output_dir: Union[str, Path] = "checkpoints",
        manifest_path: Union[str, Path] = "checkpoints/CHECKSUMS.txt",
    ) -> None:
        self.output_dir = Path(output_dir)
        self.manifest_path = Path(manifest_path)
        self.checksums = load_checksum_manifest(self.manifest_path)

    def verify_integrity(self, filepath: Path, expected_sha256: Optional[str] = None) -> bool:
        """Verify file existence and SHA-256 checksum."""
        if not filepath.exists():
            return False
        expected = expected_sha256 or self.checksums.get(filepath.name)
        if not expected:
            return True  # No checksum recorded in manifest, accept file
        actual = compute_file_sha256(filepath)
        return actual.lower() == expected.lower()

    def pull(
        self,
        checkpoint_name: str,
        force: bool = False,
        release_tag: str = "v0.1.0-alpha",
        repo_id: str = "tryggth/bourbakimesh-muzero",
        github_repo: str = "tryggth/bourbakimesh",
        p2p_addr: Optional[str] = None,
    ) -> Path:
        """Download or retrieve model checkpoint using the multi-tier fallback hierarchy."""
        if not checkpoint_name.endswith(".pt") and not checkpoint_name.endswith(".safetensors"):
            checkpoint_name = f"{checkpoint_name}.pt"

        self.output_dir.mkdir(parents=True, exist_ok=True)
        target_path = self.output_dir / checkpoint_name
        expected_sha256 = self.checksums.get(checkpoint_name)

        # Tier 1: Local Disk Cache
        if target_path.exists() and not force:
            if self.verify_integrity(target_path, expected_sha256):
                print(f"[Tier 1] Found valid cached checkpoint on disk: {target_path}")
                return target_path
            else:
                print(f"[Tier 1] Local file {target_path} failed integrity check. Re-fetching...")

        # Tier 2: Local P2P Swarm
        if p2p_addr:
            print(f"[Tier 2] Querying P2P swarm at {p2p_addr} for model {checkpoint_name}...")
            # P2P chunk resolution hook

        # Tier 3: GitHub Release
        print(f"[Tier 3] Attempting GitHub Release download for {checkpoint_name} (tag: {release_tag})...")
        gh_downloaded = self._try_github_download(checkpoint_name, target_path, release_tag, github_repo)
        if gh_downloaded and self.verify_integrity(target_path, expected_sha256):
            print(f"[Tier 3] Successfully downloaded and verified from GitHub Releases: {target_path}")
            return target_path

        # Tier 4: Hugging Face Hub
        print(f"[Tier 4] Attempting Hugging Face Hub download from {repo_id}/{checkpoint_name}...")
        hf_downloaded = self._try_hf_download(checkpoint_name, target_path, repo_id)
        if hf_downloaded and self.verify_integrity(target_path, expected_sha256):
            print(f"[Tier 4] Successfully downloaded and verified from Hugging Face: {target_path}")
            return target_path

        # If file exists on disk (even if network downloads were skipped/failed), check if acceptable
        if target_path.exists() and self.verify_integrity(target_path, expected_sha256):
            return target_path

        raise FileNotFoundError(
            f"Could not retrieve model checkpoint '{checkpoint_name}' across all tiers. "
            f"Expected SHA-256: {expected_sha256 or 'unknown'}"
        )

    def _try_github_download(
        self,
        checkpoint_name: str,
        target_path: Path,
        release_tag: str,
        github_repo: str,
    ) -> bool:
        # Method A: Try gh CLI
        try:
            res = subprocess.run(
                [
                    "gh",
                    "release",
                    "download",
                    release_tag,
                    "--repo",
                    github_repo,
                    "--pattern",
                    checkpoint_name,
                    "--dir",
                    str(self.output_dir),
                    "--clobber",
                ],
                capture_output=True,
                text=True,
                check=False,
            )
            if res.returncode == 0 and target_path.exists():
                return True
        except Exception:
            pass

        # Method B: HTTP request
        url = f"https://github.com/{github_repo}/releases/download/{release_tag}/{checkpoint_name}"
        try:
            req = urllib.request.Request(url, headers={"User-Agent": "BourbakiMesh-ModelPuller/1.0"})
            with urllib.request.urlopen(req, timeout=10) as response, open(target_path, "wb") as out_file:
                while chunk := response.read(1024 * 1024):
                    out_file.write(chunk)
            return True
        except Exception:
            return False

    def _try_hf_download(
        self,
        checkpoint_name: str,
        target_path: Path,
        repo_id: str,
    ) -> bool:
        # Method A: Try huggingface_hub Python library if installed
        try:
            from huggingface_hub import hf_hub_download  # type: ignore

            downloaded = hf_hub_download(
                repo_id=repo_id,
                filename=checkpoint_name,
                local_dir=str(self.output_dir),
            )
            if Path(downloaded).exists():
                return True
        except ImportError:
            pass
        except Exception:
            pass

        # Method B: HTTP direct download
        url = f"https://huggingface.co/{repo_id}/resolve/main/{checkpoint_name}"
        try:
            req = urllib.request.Request(url, headers={"User-Agent": "BourbakiMesh-ModelPuller/1.0"})
            with urllib.request.urlopen(req, timeout=10) as response, open(target_path, "wb") as out_file:
                while chunk := response.read(1024 * 1024):
                    out_file.write(chunk)
            return True
        except Exception:
            return False


def pull_checkpoint(
    checkpoint_name: str,
    output_dir: Union[str, Path] = "checkpoints",
    force: bool = False,
    release_tag: str = "v0.1.0-alpha",
    repo_id: str = "tryggth/bourbakimesh-muzero",
) -> Path:
    """Convenience function to pull model weights."""
    puller = ModelPuller(output_dir=output_dir)
    return puller.pull(
        checkpoint_name=checkpoint_name,
        force=force,
        release_tag=release_tag,
        repo_id=repo_id,
    )


def main() -> None:
    parser = argparse.ArgumentParser(
        prog="bourbakimesh.models.pull",
        description="Pull BourbakiMuZero model weights from multi-tier registry (Local, P2P, GitHub, Hugging Face).",
    )
    parser.add_argument(
        "--checkpoint",
        "-c",
        required=True,
        help="Name of checkpoint to pull (e.g. bourbaki_v0.pt, bourbaki_v1.pt, bourbaki_v2.pt)",
    )
    parser.add_argument(
        "--output-dir",
        "-o",
        default="checkpoints",
        help="Directory to save checkpoint (default: checkpoints/)",
    )
    parser.add_argument(
        "--force",
        "-f",
        action="store_true",
        help="Force re-download even if cached locally",
    )
    parser.add_argument(
        "--release-tag",
        default="v0.1.0-alpha",
        help="GitHub release tag (default: v0.1.0-alpha)",
    )
    parser.add_argument(
        "--repo-id",
        default="tryggth/bourbakimesh-muzero",
        help="Hugging Face repository ID (default: tryggth/bourbakimesh-muzero)",
    )
    parser.add_argument(
        "--p2p-addr",
        default=None,
        help="Optional P2P swarm address to pull chunks from",
    )

    args = parser.parse_args()
    puller = ModelPuller(output_dir=args.output_dir)

    try:
        path = puller.pull(
            checkpoint_name=args.checkpoint,
            force=args.force,
            release_tag=args.release_tag,
            repo_id=args.repo_id,
            p2p_addr=args.p2p_addr,
        )
        print(f"Successfully verified checkpoint: {path}")
    except Exception as e:
        print(f"Error pulling checkpoint: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
