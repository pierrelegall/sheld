# Changelog

## Next

- **Breaking:** Rename the local configuration file from `.sheld.yaml` to `sheld.yaml`. A `.sheld.yaml` is no longer read: finding one is an error, so its commands are not silently left unsandboxed. Rename it with `mv .sheld.yaml sheld.yaml`, or merge it into an existing `sheld.yaml`. `sheld init` now writes `sheld.yaml`.
- **Breaking:** Resolve relative paths against the configuration file that declares them, instead of the working directory. Use `$PWD` for the working directory.
- **Breaking:** Replace `env` with `setenv` or `setenv_if_unset`, depending on whether inherited values may override it.
- **Breaking:** Rename `unset_env` to `unsetenv`.
- **Breaking:** Reject unset and empty variables in configuration paths, instead of keeping them as written or expanding them to nothing.
- **Breaking:** Reject empty fallbacks (`${VAR:-}`) in configuration paths.
- Warn that no commands will be sandboxed when a configuration file fails to load, naming the file and the cause.
- Add `alias` and `args` to command entries.
- Apply mounts in a fixed order, read-only mounts after writable ones.
- Identify mount entries by destination: a later entry with the same destination replaces the earlier one.
- Use the `${VAR:-fallback}` fallback when `VAR` is empty, not only when it is unset.
- Name the variable, field, path and entry in configuration path errors.
- Report configuration load errors from `sheld check` instead of exiting silently.
- Keep `sheld list --simple` silent when no configuration exists.

## v0.1.0 (2026-03-30)

First release of Sheld. See README for more information.
