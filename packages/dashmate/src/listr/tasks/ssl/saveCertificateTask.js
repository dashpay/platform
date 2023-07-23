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
          const configDir = homeDir.joinPath('ssl', config.getName());

          fs.mkdirSync(configDir, { recursive: true });

          const crtFile = path.join(configDir, 'bundle.crt');

          fs.writeFileSync(crtFile, ctx.certificateFile, 'utf8');

          const keyFile = path.join(configDir, 'private.key');
          fs.writeFileSync(keyFile, ctx.privateKeyFile, 'utf8');

          config.set('platform.dapi.envoy.ssl.enabled', true);
        },
      }]);
  }

  return saveCertificateTask;
}

module.exports = saveCertificateTaskFactory;
