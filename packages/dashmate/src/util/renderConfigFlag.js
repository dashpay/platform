/**
 * Values a POSIX shell passes through untouched.
 */
const SHELL_SAFE = /^[A-Za-z0-9._@%+=:,/-]+$/;

/**
 * Render the `--config` flag for a command an operator is meant to copy and run.
 *
 * Omitted when the command is already about the node dashmate would act on
 * without it. Naming the default config adds nothing an operator can act on and
 * lengthens every command dashmate prints, and most nodes have exactly one.
 *
 * It is still printed for every other config, and that is not decoration: an
 * operator running several nodes who pastes a bare command obtains a
 * certificate for, restarts, or bypasses a check on a different one. When the
 * default is unknown - a collected report from another machine that predates
 * this, say - the flag is printed, because being explicit is only wasteful
 * while being wrong is not recoverable.
 *
 * Carries its own leading space, so a command reads correctly with the flag and
 * with it gone. Callers interpolate it directly against the previous word:
 * `dashmate doctor${cfg}`, never `dashmate doctor ${cfg}`.
 *
 * @param {string} configName
 * @param {string|null} [defaultConfigName]
 * @return {string}
 */
export default function renderConfigFlag(configName, defaultConfigName = null) {
  const name = String(configName);

  if (defaultConfigName !== null && defaultConfigName !== undefined && name === String(defaultConfigName)) {
    return '';
  }

  return ` --config ${SHELL_SAFE.test(name) ? name : `'${name.replace(/'/g, "'\\''")}'`}`;
}
