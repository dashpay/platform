import { Listr } from 'listr2';
import path from 'path';
import fs from 'fs';

/**
 * @param {HomeDir} homeDir
 * @return {saveCertificateTask}
 */
export default function saveCertificateTaskFactory(homeDir) {
  /**
   * @typedef {function} saveCertificateTask
   * @param {Config} config
   * @return {Listr}
   */
  function saveCertificateTask(config) {
    return new Listr([
      {
        title: 'Save certificates',
        task: async (ctx) => {
          const certificatesDir = homeDir.joinPath(
            config.getName(),
            'platform',
            'gateway',
            'ssl',
          );

          fs.mkdirSync(certificatesDir, { recursive: true });

          const crtFile = path.join(certificatesDir, 'bundle.crt');
          const keyFile = path.join(certificatesDir, 'private.key');

          // Docker bind-mounts both files into the gateway container individually,
          // and a file bind mount follows the inode rather than the path. Writing
          // in place is what lets a renewal reach the running gateway: replacing
          // either file leaves the container reading the one it mounted at
          // startup, and Envoy would serve the previous certificate until it
          // expires.
          fs.writeFileSync(crtFile, ctx.certificateFile, 'utf8');

          // Dashmate used to create this file at the process umask, so an
          // upgraded node carries a group- and world-readable private key.
          // Dropping those bits repairs it on the next renewal, while an owner
          // that hardened it further - 0400 - keeps what it chose.
          const keyExists = fs.existsSync(keyFile);
          const keyMode = keyExists
            // eslint-disable-next-line no-bitwise
            ? fs.statSync(keyFile).mode & 0o700
            : 0o600;

          // An owner who hardened the key to 0400 has removed the write bit that
          // writing in place needs, so restore it for the write and put the
          // chosen mode back straight after.
          if (keyExists) {
            fs.chmodSync(keyFile, 0o600);
          }

          fs.writeFileSync(keyFile, ctx.privateKeyFile, { encoding: 'utf8', mode: keyMode });
          fs.chmodSync(keyFile, keyMode);

          // A running gateway only picks up what was written here once it is
          // told to reload, so let callers see that the pair changed.
          ctx.certificateSaved = true;

          config.set('platform.gateway.ssl.enabled', true);
        },
      }]);
  }

  return saveCertificateTask;
}
