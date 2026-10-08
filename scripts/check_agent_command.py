"""Block hook-bypassing Git commits in ordinary agent shell commands.

This is a command guard, not an arbitrary-shell/program interpreter. CI and
required merge checks remain necessary even when an agent hook is trusted.
"""

import json
import shlex
import sys
from pathlib import Path

VALUE_OPTIONS = {
    "-m", "-F", "-t", "-C", "-c", "--message", "--file", "--template",
    "--reuse-message", "--reedit-message", "--author", "--date", "--trailer",
}


def bypass_flag(arguments):
    iterator = iter(arguments)
    for argument in iterator:
        if argument == "--":
            break
        if argument in VALUE_OPTIONS:
            next(iterator, None)
        elif argument.startswith("--no-ver") and "--no-verify".startswith(argument):
            return True
        elif argument.startswith("-") and not argument.startswith("--"):
            for letter in argument[1:]:
                if letter in "mFtCcuS":
                    break
                if letter == "n":
                    return True
    return False


def git_arguments(tokens):
    iterator = iter(tokens[1:])
    hooks_override = False
    for token in iterator:
        if token == "-c":
            config = next(iterator, "")
            hooks_override |= config.lower().startswith("core.hookspath=")
        elif token.startswith("-c"):
            hooks_override |= token[2:].lower().startswith("core.hookspath=")
        elif token.startswith("--config-env=core.hooksPath="):
            hooks_override = True
        elif token in {"-C", "--git-dir", "--work-tree", "--namespace"}:
            next(iterator, None)
        elif not token.startswith("-"):
            if token == "commit":
                return hooks_override, list(iterator)
            return False, None
    return False, None


def unwrap_command(tokens):
    while tokens:
        program = Path(tokens[0]).name
        if "=" in tokens[0] or program in {"env", "command", "time", "sudo", "nice"}:
            wrapper = program
            tokens = tokens[1:]
            while tokens and tokens[0].startswith("-"):
                option = tokens.pop(0)
                if ((wrapper == "sudo" and option in {"-u", "-g", "-h", "-p"})
                        or (wrapper == "env" and option in {"-u", "--unset"})
                        or (wrapper == "nice" and option in {"-n", "--adjustment"})):
                    tokens = tokens[1:]
            continue
        break
    return tokens


def blocked_segment(tokens):
    tokens = unwrap_command(tokens)
    if not tokens:
        return False
    program = Path(tokens[0]).name
    if program in {"sh", "bash", "zsh"} and len(tokens) > 2:
        if tokens[1] in {"-c", "-lc"}:
            return blocked_command(tokens[2])
    override, arguments = git_arguments(tokens) if program == "git" else (False, None)
    return arguments is not None and (override or bypass_flag(arguments))


def blocked_command(command):
    lexer = shlex.shlex(command, posix=True, punctuation_chars=";&|()\n")
    lexer.whitespace = " \t\r"
    lexer.whitespace_split = True
    segment = []
    for token in lexer:
        if token and all(char in ";&|()\n" for char in token):
            if blocked_segment(segment):
                return True
            segment = []
        else:
            segment.append(token)
    return blocked_segment(segment)


def main():
    try:
        payload = json.load(sys.stdin)
        tool_input = payload.get("tool_input", {})
        command = tool_input.get("command", tool_input.get("cmd", ""))
        try:
            denied = not isinstance(command, str) or blocked_command(command)
        except ValueError:
            # shlex is not a shell parser: legitimate heredoc bodies may contain
            # unmatched quotes. Allow those; CI rechecks commits independently.
            denied = False
    except (ValueError, AttributeError):
        denied = True
    if denied:
        print(json.dumps({"hookSpecificOutput": {
            "hookEventName": "PreToolUse", "permissionDecision": "deny",
            "permissionDecisionReason": "AGENTS.md requires commit hooks; fix failures instead of bypassing them.",
        }}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
