import { Listr } from 'listr2';
import path from 'path';
import fs from 'fs';

import { clearRenewalRecord } from '../../../ssl/renewalRecord.js';
import selectLeafCertificate from '../../../ssl/selectLeafCertificate.js';
import renderConfigFlag from '../../../util/renderConfigFlag.js';

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

          try {
            fs.writeFileSync(keyFile, ctx.privateKeyFile, { encoding: 'utf8', mode: keyMode });
          } finally {
            // Also runs when the write throws, so a failure cannot leave the key
            // at the looser mode it was given to make the write possible.
            if (fs.existsSync(keyFile)) {
              fs.chmodSync(keyFile, keyMode);
            }
          }

          // The two writes above are separate and in place, so a full disk, a
          // failed chmod or a power loss between them can leave a new
          // certificate paired with the old key, or a truncated bundle. With
          // the gateway stopped - the state the documented upgrade procedure
          // leaves it in - nothing else would notice: the command would report
          // success and the node would simply fail to come back up at the next
          // start, a step removed from whatever caused it.
          const { error, detail } = selectLeafCertificate(
            fs.readFileSync(crtFile, 'utf8'),
            fs.readFileSync(keyFile, 'utf8'),
          );

          if (error) {
            throw new Error(`The certificate and private key written for the gateway do not match:`
              + ` ${detail}.\n`
              + `Certificate: ${crtFile}\nPrivate key: ${keyFile}\n`
              + 'The gateway will not start with these files. Obtain the certificate again:\n'
              + `    dashmate ssl obtain ${renderConfigFlag(config.getName())} --force`);
          }

          config.set('platform.gateway.ssl.enabled', true);

          // A usable pair is installed, so any earlier failure has been
          // overtaken. The helper cannot notice this on its own: after a failed
          // renewal it stops watching the configuration until it retries an
          // hour later, and installing a certificate changes none of the values
          // it watches anyway. Without this an operator who has just repaired
          // their node is told renewal is failing, at the moment they run the
          // command to check their work.
          try {
            clearRenewalRecord(homeDir, config.getName());
          } catch (e) {
            // Bookkeeping must not fail an install. The pair is already on
            // disk and the provider is already set; throwing here would report
            // a renewal that fully succeeded as a failure.
            // eslint-disable-next-line no-console
            console.warn(`Could not clear the renewal record: ${e.message}`);
          }
        },
      }]);
  }

  return saveCertificateTask;
}
