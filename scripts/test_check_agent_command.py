"""Verify hook decisions without executing the candidate commands."""

import json
import subprocess
import sys
import unittest
from pathlib import Path

from check_agent_command import blocked_command


class AgentCommandTests(unittest.TestCase):
    def test_bypass_commands_are_blocked(self):
        for command in [
            "git commit --no-verify", "git commit -n -m change",
            "git commit --no-veri", "git commit -u -n -m x",
            "env -u FOO git commit -n", "nice -n 5 git commit -n",
            "git --config-env=core.hooksPath=HOOK_DIR commit -m x", "git -c core.hooksPath=/dev/null commit -m change",
            "sudo git commit -n", "time git commit -n", "env -i git commit -n",
            "git commit -m change --no-verify", "git commit -snm change",
            "git -C /tmp/repo commit -s -n", "git -c user.name=Alex commit -n",
            "cd /tmp/repo && git commit --no-verify",
            "echo ready\ngit commit -n", "env FLAG=1 /usr/bin/git commit -n",
            "bash -lc 'git commit --no-verify'",
        ]:
            with self.subTest(command=command):
                self.assertTrue(blocked_command(command))

    def test_config_env_hooks_path_is_case_insensitive(self):
        for key in ["core.hooksPath", "core.hookspath", "CORE.HOOKSPATH"]:
            with self.subTest(key=key):
                self.assertTrue(blocked_command(
                    f"git --config-env={key}=HOOK_DIR commit -m change"
                ))
        self.assertFalse(blocked_command(
            "git --config-env=user.name=AUTHOR_NAME commit -s -m change"
        ))

    def test_config_env_separate_token_is_consumed(self):
        for key in ["core.hooksPath", "core.hookspath", "CORE.HOOKSPATH"]:
            with self.subTest(key=key):
                self.assertTrue(blocked_command(
                    f"git --config-env {key}=HOOK_DIR commit -m change"
                ))
        self.assertFalse(blocked_command(
            "git --config-env user.name=AUTHOR_NAME commit -s -m change"
        ))
        self.assertTrue(blocked_command(
            "git --config-env user.name=AUTHOR_NAME commit -n -m change"
        ))
        self.assertFalse(blocked_command(
            "git --config-env core.hooksPath=HOOK_DIR status"
        ))

    def test_normal_commands_and_message_text_are_allowed(self):
        for command in [
            "git commit -s -m change", "git commit -m '--no-verify'",
            "git commit -m '-n'", "git commit -mbanana", "git status -n",
            "git commit -- -n", "echo git commit --no-verify",
            "git commit -uno", "git commit -u no", "git commit -Skeyn -m x",
        ]:
            with self.subTest(command=command):
                self.assertFalse(blocked_command(command))

    def test_codex_hook_protocol_denies_without_running_command(self):
        script = Path(__file__).with_name("check_agent_command.py")
        result = subprocess.run(
            [sys.executable, str(script)],
            input=json.dumps({"tool_input": {"command": "git commit -n"}}),
            capture_output=True, text=True, check=True,
        )
        output = json.loads(result.stdout)["hookSpecificOutput"]
        self.assertEqual(output["hookEventName"], "PreToolUse")
        self.assertEqual(output["permissionDecision"], "deny")
    def test_legitimate_heredoc_with_apostrophe_falls_through(self):
        script = Path(__file__).with_name("check_agent_command.py")
        command = "git commit -F - <<'EOF'\nFix the maintainer's docs\nEOF\n"
        result = subprocess.run(
            [sys.executable, str(script)],
            input=json.dumps({"tool_input": {"command": command}}),
            capture_output=True, text=True, check=True,
        )
        self.assertEqual(result.stdout, "")


if __name__ == "__main__":
    unittest.main()
