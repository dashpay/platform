import path from 'path';

export const CONFIG_NAME_PATTERN = /^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$/u;

/**
 * Names whose directory would sit on top of a file the repository owns.
 *
 * Everything the repository writes alongside the config file is dot-prefixed,
 * which no config name can reach, so only the config file itself remains. A
 * name is still compared case-insensitively and without trailing periods,
 * because Windows aliases those and common macOS filesystems fold case.
 */
const RESERVED_CONFIG_NAMES = new Set([
  'config.json',
]);

/**
 * Require a config or group name to be one portable path-safe segment.
 *
 * Rejecting a reserved name is checked separately, on the way in. A config file
 * written before this rule existed has to stay loadable, or the collection
 * cannot be read at all and there is no way to run the command that would
 * remove the offending entry.
 *
 * @param {string} name
 */
export function assertSafeConfigName(name) {
  if (typeof name !== 'string' || !CONFIG_NAME_PATTERN.test(name)) {
    throw new Error('Config name must be one path-safe segment');
  }
}

/**
 * Reject a name Dashmate cannot give a directory to.
 *
 * Applied where a name is chosen - creating a config or a group - rather than
 * where one is loaded.
 *
 * @param {string} name
 */
export function isConfigNameAvailable(name) {
  return !RESERVED_CONFIG_NAMES.has(name.replace(/\.+$/u, '').toLowerCase());
}

/**
 * Reject a name Dashmate cannot give a directory to.
 *
 * Applied where a name is chosen - creating a config or a group - rather than
 * where one is loaded.
 *
 * @param {string} name
 */
export function assertConfigNameAvailable(name) {
  assertSafeConfigName(name);

  const canonicalName = name.replace(/\.+$/u, '').toLowerCase();

  if (RESERVED_CONFIG_NAMES.has(canonicalName)) {
    throw new Error(`Config name '${name}' is reserved by Dashmate`);
  }
}

/**
 * Resolve a configuration directory and prove it is a direct child of the
 * Dashmate home directory.
 *
 * @param {HomeDir} homeDir
 * @param {string} name
 * @returns {string}
 */
export default function resolveConfigDirectory(homeDir, name) {
  assertSafeConfigName(name);

  const root = path.resolve(homeDir.getPath());
  const target = path.resolve(root, name);

  if (target === root || path.dirname(target) !== root) {
    throw new Error('Config directory must be a direct child of Dashmate home');
  }

  return target;
}
