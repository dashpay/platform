import path from 'path';

/**
 * Everything a renewal has to write, for a given config.
 *
 * Kept in one place because the answer is asked from three: the doctor collects
 * it, `update` checks it live, and a test has to be able to break exactly what
 * a renewal would touch. Two of them drifting apart would mean one surface
 * clearing a node the other refuses.
 *
 * @param {HomeDir} homeDir
 * @param {string} configName
 * @return {{directories: string[], files: string[]}}
 */
export default function certificateStorageTargets(homeDir, configName) {
  const legoDir = homeDir.joinPath(configName, 'platform', 'gateway', 'lego');
  const sslDir = homeDir.joinPath(configName, 'platform', 'gateway', 'ssl');

  return {
    // Where the helper writes what the authority returns.
    directories: [path.join(legoDir, 'certificates'), sslDir],
    // The two the gateway already has bind-mounted. They are overwritten in
    // place rather than replaced, so their own permissions decide, not their
    // directory's.
    files: [path.join(sslDir, 'bundle.crt'), path.join(sslDir, 'private.key')],
  };
}
