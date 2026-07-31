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
          const crtTempFile = `${crtFile}.tmp-${process.pid}`;
          const keyTempFile = `${keyFile}.tmp-${process.pid}`;
          const previousCertificate = fs.existsSync(crtFile)
            ? fs.readFileSync(crtFile)
            : null;
          let certificateReplaced = false;

          try {
            fs.writeFileSync(crtTempFile, ctx.certificateFile, 'utf8');
            fs.writeFileSync(keyTempFile, ctx.privateKeyFile, 'utf8');
            fs.renameSync(crtTempFile, crtFile);
            certificateReplaced = true;
            fs.renameSync(keyTempFile, keyFile);
          } catch (e) {
            if (certificateReplaced) {
              if (previousCertificate === null) {
                fs.rmSync(crtFile, { force: true });
              } else {
                fs.writeFileSync(crtFile, previousCertificate);
              }
            }

            throw e;
          } finally {
            fs.rmSync(crtTempFile, { force: true });
            fs.rmSync(keyTempFile, { force: true });
          }

          config.set('platform.gateway.ssl.enabled', true);
        },
      }]);
  }

  return saveCertificateTask;
}
