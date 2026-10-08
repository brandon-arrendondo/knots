# Commit command deny rules

`settings.json` denies ordinary hook-bypassing commits, including `-nm` and
`-snm`. Patterns match command text: a message such as `git commit -m "fix -n
flag"` may also be denied. Use a message file for that legitimate case.
These patterns do not interpret every possible flag bundle or arbitrary
program that invokes Git. CI checks commit messages and reruns hooks.

The Codex guard handles `env -u FOO git commit -n`, ordinary `nice` wrappers,
and `--config-env=core.hooksPath=...` on commits. Attached `-S` signing-key
values are consumed, so `-Skeyn` is not mistaken for a bypass flag.
Remaining limits include Git configuration injected through `GIT_CONFIG_COUNT`
and `GIT_CONFIG_KEY_n`/`GIT_CONFIG_VALUE_n`, other wrapper/alias forms, and a
heredoc in the same command: if shlex cannot parse it, the whole command falls
through to allow. CI remains the independent check.
