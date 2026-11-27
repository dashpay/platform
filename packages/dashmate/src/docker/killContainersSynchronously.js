import { spawnSync as nodeSpawnSync } from 'child_process';

/**
 * Kill containers before returning, for a process that is about to exit
 *
 * listr2 answers SIGINT with process.exit(), and after that only synchronous
 * work runs, so a Docker API request started then is never sent. The CLI call
 * here blocks until Docker has killed them. Failures are ignored: the
 * asynchronous cleanup that follows reports what it can.
 *
 * @typedef {function} killContainersSynchronously
 * @param {string[]} containerIds
 * @param {function} [spawnSync]
 */
export default function killContainersSynchronously(containerIds, spawnSync = nodeSpawnSync) {
  if (containerIds.length === 0) {
    return;
  }

  spawnSync('docker', ['kill', ...containerIds], { stdio: 'ignore', timeout: 10000 });
}
