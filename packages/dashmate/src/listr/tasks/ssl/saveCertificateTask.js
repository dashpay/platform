const { Listr } = require('listr2');
const path = require('path');
const fs = require('fs');

/**
 * @param {HomeDir} homeDir
 * @return {saveCertificateTask}
 */
function saveCertificateTaskFactory(homeDir) {
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
          const certificatesDir = homeDir.joinPath(config.getName(),
            'platform', 'dapi', 'envoy', 'ssl');

          fs.mkdirSync(certificatesDir, { recursive: true });

          const crtFile = path.join(certificatesDir, 'bundle.crt');

          fs.writeFileSync(crtFile, ctx.certificateFile, 'utf8');

          const keyFile = path.join(certificatesDir, 'private.key');
          fs.writeFileSync(keyFile, ctx.privateKeyFile, 'utf8');

          config.set('platform.dapi.envoy.ssl.enabled', true);
        },
      }]);
  }

  return saveCertificateTask;
}

module.exports = saveCertificateTaskFactory;
