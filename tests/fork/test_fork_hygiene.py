#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
test_fork_hygiene.py

Fork 衛生與治理規則檢查：
驗證維護文件完整性、雙語 README、UTF-8 編碼、Git remote 設定、以及 CI 發布守衛。
"""

import os
import unittest
import subprocess
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]


class TestForkHygiene(unittest.TestCase):
    def test_required_fork_documents_exist(self):
        required_files = [
            "FORK.md",
            "NOTICE.md",
            "AGENTS.md",
            "README.md",
            "README.en.md",
            "docs/agent-desktop-computer-use-spec.md",
            "docs/fork/UPSTREAM.md",
            "docs/fork/DECISIONS.md",
            ".cursor/rules/no-upstream-pr.mdc",
            "tools/dev_check.ps1",
        ]
        for rel_path in required_files:
            file_path = REPO_ROOT / rel_path
            self.assertTrue(file_path.is_file(), f"Missing required fork document: {rel_path}")

    def test_bilingual_readme_links(self):
        readme_zh = (REPO_ROOT / "README.md").read_text(encoding="utf-8")
        readme_en = (REPO_ROOT / "README.en.md").read_text(encoding="utf-8")

        self.assertIn("README.en.md", readme_zh, "README.md 應包含切換至 README.en.md 的連結")
        self.assertIn("README.md", readme_en, "README.en.md 應包含切換至 README.md 的連結")

    def test_all_fork_files_are_valid_utf8(self):
        check_files = [
            "FORK.md",
            "NOTICE.md",
            "AGENTS.md",
            "README.md",
            "README.en.md",
            "docs/agent-desktop-computer-use-spec.md",
            "docs/fork/UPSTREAM.md",
            "docs/fork/DECISIONS.md",
            ".cursor/rules/no-upstream-pr.mdc",
            "tools/dev_check.ps1",
        ]
        for rel_path in check_files:
            path = REPO_ROOT / rel_path
            if path.is_file():
                try:
                    content = path.read_text(encoding="utf-8")
                    self.assertGreater(len(content), 10, f"{rel_path} is unexpectedly short")
                except UnicodeDecodeError as err:
                    self.fail(f"File {rel_path} is not valid UTF-8: {err}")

    def test_workflow_release_guard(self):
        release_yml = REPO_ROOT / ".github" / "workflows" / "release.yml"
        if release_yml.is_file():
            content = release_yml.read_text(encoding="utf-8")
            self.assertIn(
                "vercel-labs/agent-desktop",
                content,
                "release.yml should include a repository guard check for vercel-labs/agent-desktop"
            )

    def test_governance_no_upstream_push(self):
        fork_md = (REPO_ROOT / "FORK.md").read_text(encoding="utf-8")
        agents_md = (REPO_ROOT / "AGENTS.md").read_text(encoding="utf-8")
        rule_mdc = (REPO_ROOT / ".cursor" / "rules" / "no-upstream-pr.mdc").read_text(encoding="utf-8")

        self.assertIn("SanHsien/agent-desktop", fork_md)
        self.assertIn("SanHsien/agent-desktop", agents_md)
        self.assertIn("SanHsien/agent-desktop", rule_mdc)


if __name__ == "__main__":
    unittest.main()
