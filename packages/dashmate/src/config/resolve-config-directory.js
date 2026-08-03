import path from 'path';

export const CONFIG_NAME_PATTERN = /^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$/u;

/**
 * Require a config or group name to be one portable path-safe segment.
 *
 * @param {string} name
 */
export function assertSafeConfigName(name) {
  if (typeof name !== 'string' || !CONFIG_NAME_PATTERN.test(name)) {
    throw new Error('Config name must be one path-safe segment');
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
