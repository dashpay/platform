import quoteShellArgument from '../util/quoteShellArgument.js';
import renderConfigFlag from '../util/renderConfigFlag.js';

export const NODE_KEY_PATH = 'platform.drive.tenderdash.node.key';

/**
 * The commands that back up a masternode's Platform node key before a hard
 * Platform reset, and that give it to setup afterwards
 *
 * The key is the one the registration names, and dashmate keeps no copy of it,
 * so the backup must not cost an earlier one:
 * - the file is named after the config, so backups of several nodes in one
 *   directory do not collide;
 * - `set -C` makes the redirection refuse a file that exists. Without it the
 *   shell truncates the file before the export runs, so even a failed export
 *   loses the earlier backup, and an existing file keeps its mode;
 * - `umask 077` creates the file readable by its owner only. It stays inside
 *   the subshell: left in the operator's shell, it would give the files the
 *   reset renders next modes the Platform containers cannot read.
 *
 * Config names are not restricted, so the file name keeps only characters
 * that cannot change the path.
 *
 * @param {string} configName
 * @return {{ fileName: string, backupCommand: string, setupCommand: string }}
 *  fileName is quoted for the shell
 */
export default function renderNodeKeyBackup(configName) {
  const flag = renderConfigFlag(configName);
  const fileName = quoteShellArgument(`node-key-${configName.replace(/[^A-Za-z0-9._-]/g, '_')}`);

  return {
    fileName,
    backupCommand: `(umask 077 && set -C && dashmate config get ${NODE_KEY_PATH} ${flag} > ${fileName})`,
    setupCommand: `dashmate setup --platform ${flag} --platform-node-key-file ${fileName}`,
  };
}
