import { Listr } from 'listr2';
import path from 'path';
import fs from 'fs';
import graceful from 'node-graceful';

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

          fs.readdirSync(certificatesDir)
            .filter((fileName) => (
              fileName.startsWith('bundle.crt.tmp-')
              || fileName.startsWith('private.key.tmp-')
            ))
            .forEach((fileName) => {
              fs.rmSync(path.join(certificatesDir, fileName), { force: true });
            });

          const crtTempFile = `${crtFile}.tmp-${process.pid}`;
          const keyTempFile = `${keyFile}.tmp-${process.pid}`;
          const previousCertificate = fs.existsSync(crtFile)
            ? fs.readFileSync(crtFile)
            : null;
          const certificateMode = fs.existsSync(crtFile)
            // eslint-disable-next-line no-bitwise
            ? fs.statSync(crtFile).mode & 0o777
            : 0o644;
          // Dashmate used to create this file at the process umask, so an
          // upgraded node carries a group- and world-readable private key.
          // Dropping those bits repairs it on the next renewal, while an owner
          // that hardened it further - 0400 - keeps what it chose.
          const keyMode = fs.existsSync(keyFile)
            // eslint-disable-next-line no-bitwise
            ? fs.statSync(keyFile).mode & 0o700
            : 0o600;
          let certificateReplaced = false;
          const cleanupTempFiles = () => {
            fs.rmSync(crtTempFile, { force: true });
            fs.rmSync(keyTempFile, { force: true });
          };
          const unsubscribe = graceful.on('exit', cleanupTempFiles);

          try {
            fs.writeFileSync(crtTempFile, ctx.certificateFile, {
              encoding: 'utf8',
              mode: certificateMode,
            });
            fs.chmodSync(crtTempFile, certificateMode);
            fs.writeFileSync(keyTempFile, ctx.privateKeyFile, {
              encoding: 'utf8',
              mode: keyMode,
            });
            fs.chmodSync(keyTempFile, keyMode);
            fs.renameSync(crtTempFile, crtFile);
            certificateReplaced = true;
            fs.renameSync(keyTempFile, keyFile);
          } catch (e) {
            if (certificateReplaced) {
              if (previousCertificate === null) {
                fs.rmSync(crtFile, { force: true });
              } else {
                fs.writeFileSync(crtFile, previousCertificate, { mode: certificateMode });
                fs.chmodSync(crtFile, certificateMode);
              }
            }

            throw e;
          } finally {
            cleanupTempFiles();
            unsubscribe();
          }

          config.set('platform.gateway.ssl.enabled', true);
        },
      }]);
  }

  return saveCertificateTask;
}
