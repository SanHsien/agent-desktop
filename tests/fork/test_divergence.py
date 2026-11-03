"""Contracts for the product divergence ledger and its read-only checker."""

import contextlib
import importlib
import io
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT / "tools"))


class GovernanceRegression(unittest.TestCase):
    def test_adr_002_does_not_claim_a_missing_local_tool(self):
        text = (REPO_ROOT / "docs/fork/DECISIONS.md").read_text("utf-8")
        section = text.split("## ADR-002:", 1)[1].split("\n## ADR-", 1)[0]
        for name in re.findall(r"`(tools/[^`]+\.py)`", section):
            self.assertTrue((REPO_ROOT / name).is_file(), name)

    def test_divergence_checker_is_delivered(self):
        self.assertTrue((REPO_ROOT / "tools/check_divergence.py").is_file())
        self.assertTrue((REPO_ROOT / "docs/DIVERGENCE.md").is_file())


class DivergenceContract(unittest.TestCase):
    def setUp(self):
        self.checker = importlib.import_module("check_divergence")
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.repo = Path(self.temp.name)
        (self.repo / "docs").mkdir()
        self.document = self.repo / "docs/DIVERGENCE.md"
        self.write_ledger()

    def write_ledger(self, rows=None):
        if rows is None:
            rows = ["| `src/a.rs` | preserve behavior | upstream equivalent passes |"]
        self.document.write_text(
            "# Divergence\n\nBase: `" + "a" * 40 + "`\n\n## Product files\n\n"
            "| Path | Reason | Removal condition |\n| --- | --- | --- |\n"
            + "\n".join(rows) + "\n", encoding="utf-8"
        )

    def run_checker(self, changed="src/a.rs\0", untracked="", extra=(), fail=False):
        def git(args, **kwargs):
            self.assertEqual(kwargs["cwd"], self.repo)
            self.assertIn(args[1], {"diff", "ls-files", "rev-parse"})
            if fail:
                raise subprocess.CalledProcessError(128, args, stderr="missing ref")
            output = "a" * 40 if args[1] == "rev-parse" else (
                changed if args[1] == "diff" else untracked
            )
            return subprocess.CompletedProcess(args, 0, output, "")

        output = io.StringIO()
        with patch.object(self.checker.subprocess, "run", side_effect=git) as calls:
            with contextlib.redirect_stdout(output), contextlib.redirect_stderr(output):
                code = self.checker.main(["--repo", str(self.repo), *extra])
        return code, output.getvalue(), [call.args[0] for call in calls.call_args_list]

    def test_exact_ledger_passes_and_disables_rename_detection(self):
        code, _, calls = self.run_checker()
        self.assertEqual(code, 0)
        diff = next(args for args in calls if args[1] == "diff")
        self.assertIn("--no-renames", diff)
        self.assertEqual(diff[-4:], ["crates", "src", "apps", "npm"])

    def test_unlisted_and_obsolete_paths_both_fail(self):
        code, output, _ = self.run_checker(changed="src/b.rs\0")
        self.assertEqual(code, 1)
        self.assertIn("unlisted: src/b.rs", output)
        self.assertIn("obsolete: src/a.rs", output)

    def test_worktree_includes_untracked_product_paths(self):
        code, output, _ = self.run_checker(untracked="apps/new file.ts\0")
        self.assertEqual(code, 1)
        self.assertIn("apps/new file.ts", output)

    def test_head_mode_matches_commit_diff_without_untracked_files(self):
        code, _, calls = self.run_checker(extra=("--target", "HEAD"))
        self.assertEqual(code, 0)
        self.assertFalse(any(args[1] == "ls-files" for args in calls))
        self.assertTrue(any("HEAD^{commit}" in args for args in calls))
        diff = next(args for args in calls if args[1] == "diff")
        self.assertEqual(diff.count("a" * 40), 2)

    def test_blank_reasons_conditions_duplicates_and_bad_paths_fail(self):
        for row in (
            "| `src/a.rs` | | upstream equivalent |",
            "| `src/a.rs` | preserve behavior | |",
            "| `tools/a.py` | reason | condition |",
            "| `src/../a.rs` | reason | condition |",
        ):
            with self.subTest(row=row):
                self.write_ledger([row])
                self.assertEqual(self.run_checker()[0], 2)
        row = "| `src/a.rs` | reason | condition |"
        self.write_ledger([row, row])
        self.assertEqual(self.run_checker()[0], 2)

    def test_missing_document_and_git_failure_are_incomplete(self):
        self.assertEqual(self.run_checker(fail=True)[0], 2)
        self.document.rename(self.document.with_suffix(".bak"))
        self.assertEqual(self.run_checker()[0], 2)

    def test_empty_ledger_cannot_hide_missing_rows(self):
        self.write_ledger([])
        self.assertEqual(self.run_checker()[0], 1)

    def test_document_table_is_not_silently_skipped(self):
        self.document.write_text("# no table\n", encoding="utf-8")
        self.assertEqual(self.run_checker()[0], 2)

    def test_default_base_is_the_pinned_base_even_when_upstream_is_ahead(self):
        pinned, tip = "b" * 40, "c" * 40
        text = self.document.read_text("utf-8").replace("a" * 40, pinned)
        seen = []

        def git(repo, *args):
            seen.append(args)
            ref = args[-1]
            if "upstream" in ref:
                return tip + "\n"
            return ref.split("^")[0] + "\n"

        with patch.object(self.checker, "run_git", side_effect=git):
            self.assertEqual(self.checker.resolve_base(self.repo, None, text), pinned)
            self.assertFalse(any("upstream" in arg for a in seen for arg in a))
            self.assertEqual(
                self.checker.resolve_base(self.repo, "explicit", text), "explicit"
            )
        failure = subprocess.CalledProcessError(128, ["git"], stderr="missing")
        with patch.object(self.checker, "run_git", side_effect=failure) as git:
            with self.assertRaises(subprocess.CalledProcessError):
                self.checker.resolve_base(self.repo, "missing", text)
            self.assertEqual(git.call_count, 1)

    def test_missing_pinned_base_is_incomplete_not_a_fallback(self):
        with self.assertRaises(ValueError):
            self.checker.resolve_base(self.repo, None, "no base here")


class DivergenceWiring(unittest.TestCase):
    def test_developer_gate_checks_divergence_and_fails_closed(self):
        text = (REPO_ROOT / "tools/dev_check.ps1").read_text("utf-8")
        after = text.split("python tools/check_divergence.py", 1)[1]
        self.assertRegex(after, r'(?s)^\s*if \(\$LASTEXITCODE -ne 0\) \{\s*throw')

    def test_ci_fetches_the_pinned_baseline_history(self):
        text = (REPO_ROOT / ".github/workflows/fork-maintenance.yml").read_text("utf-8")
        self.assertIn("fetch-depth: 0", text)

    def test_checked_in_ledger_matches_product_diff(self):
        result = subprocess.run(
            [sys.executable, str(REPO_ROOT / "tools/check_divergence.py")],
            cwd=REPO_ROOT, capture_output=True, text=True, encoding="utf-8",
            timeout=30,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
