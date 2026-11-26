import quoteShellArgument from '../util/quoteShellArgument.js';
import renderConfigFlag from '../util/renderConfigFlag.js';

export const NODE_KEY_PATH = 'platform.drive.tenderdash.node.key';

/**
 * The commands that back up a masternode's Platform node key before a hard
 * Platform reset, and that give it to setup afterwards
 *
 * The key is the one the registration names, and dashmate keeps no copy of it,
 * so the backup must neither cost an earlier one nor reach anyone else:
 * - it goes into a new directory in the home directory, named after the
 *   config: the operator owns that parent, the reset removes nothing there,
 *   and setup finds it wherever it is run;
 * - `mkdir` creates that directory or fails on any entry of that name, a FIFO
 *   or a link included, which the shell's noclobber lets through. `-m 700`
 *   makes it the owner's alone at creation, and also masks the entries a
 *   default ACL of the home directory would give it;
 * - `umask 077` creates the file inside readable by its owner only, and
 *   noclobber keeps the redirection from writing into a file that exists. The
 *   umask stays inside the subshell: left in the operator's shell, it would
 *   give the files the reset renders next modes the Platform containers
 *   cannot read.
 *
 * Config names are not restricted, so the directory name keeps only
 * characters that cannot change the path.
 *
 * @param {string} configName
 * @return {{ filePath: string, backupCommand: string, setupCommand: string }}
 *  filePath is written for the shell
 */
export default function renderNodeKeyBackup(configName) {
  const flag = renderConfigFlag(configName);
  const dirPath = `"$HOME"/${quoteShellArgument(`node-key-${configName.replace(/[^A-Za-z0-9._-]/g, '_')}`)}`;
  const filePath = `${dirPath}/node-key`;

  return {
    filePath,
    backupCommand: `(umask 077 && mkdir -m 700 ${dirPath} && set -o noclobber`
      + ` && dashmate config get ${NODE_KEY_PATH} ${flag} > ${filePath})`,
    setupCommand: `dashmate setup --platform ${flag} --platform-node-key-file ${filePath}`,
  };
}
