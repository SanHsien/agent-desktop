"""Contract: the isolated-HOME test wrapper must not leak ambient state overrides."""

import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
WRAPPER = REPO_ROOT / "scripts/cargo-test-isolated-home.sh"
OVERRIDES = (
    "AGENT_DESKTOP_HOME",
    "AGENT_DESKTOP_SESSION",
    "AGENT_DESKTOP_AGENT_ID",
    "AGENT_DESKTOP_MCP_ALLOW",
    "AGENT_DESKTOP_MCP_FILE_ROOT",
)


def find_bash():
    """A working bash; on Windows PATH may hold the WSL launcher, so try Git Bash too."""
    candidates = [shutil.which("bash"), r"C:\Program Files\Git\bin\bash.exe"]
    for exe in candidates:
        if not exe or not Path(exe).exists():
            continue
        probe = subprocess.run([exe, "-c", "echo ok"], capture_output=True, text=True,
                               encoding="utf-8", errors="replace")
        if probe.returncode == 0 and probe.stdout.strip() == "ok":
            return exe
    return None


BASH = find_bash()


@unittest.skipUnless(BASH, "a working bash is required")
class IsolatedTestWrapper(unittest.TestCase):
    def test_caller_overrides_never_reach_cargo(self):
        # AGENT_DESKTOP_HOME outranks HOME for the state root, so a developer
        # machine that sets it made core tests share the real state directory.
        with tempfile.TemporaryDirectory() as tmp:
            fake = Path(tmp) / "cargo"
            fake.write_text(
                "#!/usr/bin/env bash\n"
                + "".join(f'echo "{n}=${{{n}-<unset>}}"\n' for n in OVERRIDES)
                + 'echo "HOME=$HOME"\n',
                newline="\n",
            )
            fake.chmod(0o755)
            env = dict(os.environ, HOME=tmp, PATH=tmp + os.pathsep + os.environ["PATH"])
            env.update({n: "/leaked" for n in OVERRIDES})
            out = subprocess.run(
                [BASH, WRAPPER.as_posix(), "test"],
                env=env, capture_output=True, text=True, encoding="utf-8", check=True,
            ).stdout
        for n in OVERRIDES:
            self.assertIn(f"{n}=<unset>", out, n)
        self.assertNotIn(f"HOME={tmp}\n", out)


if __name__ == "__main__":
    unittest.main()
