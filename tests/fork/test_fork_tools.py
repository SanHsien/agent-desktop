#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Contract tests for the fork-owned maintenance tools (not the agent-desktop product).

Each test names the failure it stops. Nothing here reaches the network: the registries and
`gh` are replaced, so the suite runs the same on a runner without a token.
"""

import contextlib
import io
import json
import re
import sys
import tempfile
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT / "tools"))

import check_dependency_freshness as fresh  # noqa: E402
import check_upstream_updates as upstream  # noqa: E402

WORKFLOWS = REPO_ROOT / ".github" / "workflows"


class UpstreamCheckWiring(unittest.TestCase):
    def test_baseline_points_at_vercel_labs_with_all_three_watermarks(self):
        baseline = upstream.load_baseline()
        self.assertEqual(baseline["repo"], "https://github.com/vercel-labs/agent-desktop.git")
        self.assertRegex(baseline["reviewed_through"], r"^[0-9a-f]{40}$")
        self.assertIsInstance(baseline["reviewed_pr_through"], int)
        self.assertIsInstance(baseline["reviewed_issue_through"], int)

    def test_baseline_decision_log_names_a_file_that_exists(self):
        log = upstream.load_baseline().get("decision_log", upstream.DEFAULT_DECISION_LOG)
        self.assertTrue((REPO_ROOT / log).is_file(), f"report tells people to record in {log}")

    def test_upstream_workflow_gives_gh_a_token_and_fails_closed(self):
        text = (WORKFLOWS / "upstream-check.yml").read_text(encoding="utf-8")
        self.assertIn("GH_TOKEN: ${{ github.token }}", text)
        self.assertIn("--strict", text)
        self.assertIn("fetch-depth: 0", text)

    def test_nothing_in_the_tools_pushes_to_a_remote(self):
        for name in ("check_upstream_updates.py", "check_dependency_freshness.py"):
            text = (REPO_ROOT / "tools" / name).read_text(encoding="utf-8")
            self.assertNotRegex(text, r"""["']push["']""", f"{name} must never push anywhere")


class RoadmapFormat(unittest.TestCase):
    """The fleet audit only accepts a roadmap written in the fleet format (PLAYBOOK section 8)."""

    def setUp(self):
        self.text = (REPO_ROOT / "ROADMAP.md").read_text(encoding="utf-8")

    def test_roadmap_has_the_five_fleet_headings(self):
        for heading in ("## 現況", "## 近期", "## 中期", "## 刻意不做", "## 維護方式"):
            self.assertTrue(
                any(line.startswith(heading) for line in self.text.splitlines()), heading
            )

    def test_near_and_mid_term_tables_have_the_three_required_columns(self):
        for column in ("驗收方式", "建議執行者", "證據"):
            self.assertGreaterEqual(self.text.count(column), 2, column)

    def test_every_executor_is_one_of_the_six_allowed_values(self):
        allowed = {"Claude Opus", "Claude Sonnet", "Claude Haiku", "Codex", "Antigravity", "維護者"}
        for section in ("## 近期", "## 中期"):
            body = self.text.split(section, 1)[1].split("\n## ", 1)[0]
            rows = [r for r in body.splitlines() if r.startswith("| ") and "---" not in r][1:]
            self.assertTrue(rows, section)
            for row in rows:
                cells = [c.strip() for c in row.strip("|").split(" | ")]
                self.assertIn(cells[3], allowed, row[:60])


class FreshnessLoaders(unittest.TestCase):
    def test_every_declared_manifest_exists_in_the_tree(self):
        for name in fresh.CARGO_MANIFESTS:
            self.assertTrue((REPO_ROOT / name).is_file(), name)

    def test_cargo_loader_reads_workspace_and_target_gated_dependencies(self):
        names = {p["name"] for p in fresh.load_cargo()}
        self.assertIn("clap", names, "[workspace.dependencies] is where most versions live")
        self.assertIn("uiautomation", names, "target-gated Windows dependency")
        self.assertIn("zbus", names, "exact-pinned Linux dependency")
        self.assertNotIn("agent-desktop-core", names, "a path dependency has no registry release")

    def test_cargo_member_crates_are_not_silently_dropped(self):
        sources = {p["source"] for p in fresh.load_cargo()}
        self.assertTrue({"crates/windows/Cargo.toml", "crates/linux/Cargo.toml"} <= sources)

    def test_workflow_pins_are_read_with_their_declared_version(self):
        actions = {p["name"]: p["minimum"] for p in fresh.load_workflow_actions()}
        self.assertRegex(actions["actions/checkout"], r"^\d+\.\d+")

    def test_dependabot_covers_every_ecosystem_the_freshness_check_reads(self):
        text = (REPO_ROOT / ".github" / "dependabot.yml").read_text(encoding="utf-8")
        declared = set(re.findall(r"package-ecosystem:\s*(\S+)", text))
        self.assertIn("github-actions", declared)
        self.assertTrue(set(fresh.ECOSYSTEMS) <= declared)


class FreshnessComparison(unittest.TestCase):
    def test_exact_pin_compares_every_segment_written(self):
        self.assertEqual(fresh.range_floor("=2.6.0", True), "2.6.0")
        self.assertTrue(fresh.is_newer_version("2.6.1", "2.6.0"))

    def test_caret_floor_commits_to_the_compatible_series(self):
        self.assertEqual(fresh.range_floor("4.6", True), "4")
        self.assertFalse(fresh.is_newer_version("4.9.9", "4"))
        self.assertTrue(fresh.is_newer_version("5.0.0", "4"))

    def test_zero_major_floor_commits_to_major_and_minor(self):
        self.assertEqual(fresh.range_floor("0.23", True), "0.23")
        self.assertTrue(fresh.is_newer_version("0.24.0", "0.23"))
        self.assertFalse(fresh.is_newer_version("0.23.9", "0.23"))

    def test_path_dependency_has_no_floor_instead_of_a_wrong_one(self):
        self.assertEqual(fresh.range_floor("workspace:*", True), "")

    def _row(self, latest, deferrals):
        package = {
            "name": "pkg", "minimum": "1.1", "requirement": "pkg = 1.1",
            "source": "x", "hold": "", "kind": "cargo",
        }
        return fresh.collect_status([package], lambda _n: latest, deferrals)[0]

    def test_a_deferral_expires_once_the_registry_passes_the_reviewed_release(self):
        deferrals = {"pkg": ("1.13", "not this month")}
        self.assertTrue(self._row("1.13", deferrals)["deferred_reason"])
        expired = self._row("1.13.4", deferrals)
        self.assertEqual(expired["deferred_reason"], "")
        self.assertTrue(fresh.needs_review(expired))

    def test_a_deferral_without_a_readable_release_is_ignored_not_permanent(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "deferrals.json"
            path.write_text(
                json.dumps({"deferrals": {
                    "good": {"deferredLatest": "1.2.3", "reason": "r"},
                    "forever": {"deferredLatest": "latest", "reason": "r"},
                    "silent": {"deferredLatest": "1.0.0", "reason": ""},
                }}),
                encoding="utf-8",
            )
            self.assertEqual(set(fresh.load_deferrals(path)), {"good"})

    def test_checked_in_deferrals_each_carry_a_release_and_a_reason(self):
        raw = json.loads((REPO_ROOT / ".github" / "dependency-deferrals.json").read_text("utf-8"))
        for name, entry in raw["deferrals"].items():
            self.assertTrue(entry.get("reason", "").strip(), f"{name} needs a reason")
            self.assertIsNotNone(fresh.release_key(entry.get("deferredLatest", "")), name)
        self.assertEqual(set(fresh.load_deferrals()), set(raw["deferrals"]))


class FreshnessExitCodes(unittest.TestCase):
    def _run(self, rows, argv):
        report = Path(tempfile.mkdtemp()) / "report.md"
        original = (fresh.collect_sections, sys.argv)
        fresh.collect_sections = lambda *_a: {"cargo": rows}
        sys.argv = ["x", "--output", str(report), *argv]
        try:
            with contextlib.redirect_stdout(io.StringIO()):
                return fresh.main()
        finally:
            fresh.collect_sections, sys.argv = original

    @staticmethod
    def _row(**over):
        row = {
            "name": "pkg", "source": "Cargo.toml", "requirement": "pkg = 1", "latest": "2.0.0",
            "hold": "", "outdated": True, "check_failed": False, "no_floor": False,
            "deferred_reason": "", "minimum": "1", "kind": "cargo",
        }
        row.update(over)
        return row

    def test_an_aged_floor_fails_only_under_strict(self):
        self.assertEqual(self._run([self._row()], []), 0)
        self.assertEqual(self._run([self._row()], ["--strict"]), 1)

    def test_a_held_or_deferred_floor_does_not_fail_strict(self):
        self.assertEqual(self._run([self._row(hold="standing policy")], ["--strict"]), 0)
        self.assertEqual(self._run([self._row(deferred_reason="not now")], ["--strict"]), 0)

    def test_a_registry_lookup_that_failed_is_not_reported_as_up_to_date(self):
        row = self._row(outdated=False, check_failed=True, latest="unknown")
        self.assertEqual(self._run([row], ["--strict"]), 1)

    def test_an_empty_report_is_not_a_pass(self):
        self.assertEqual(self._run([], ["--strict"]), 1)


if __name__ == "__main__":
    unittest.main()
