const { Listr } = require('listr2');
const path = require('path');
const fs = require('fs');
const { HOME_DIR_PATH } = require('../../../constants');

/**
 * @typedef {saveCertificateTask}
 * @param {Config} config
 * @return {Listr}
 */
function saveCertificateTask(config) {
  return new Listr([
    {
      title: 'Save certificates',
      task: async (ctx) => {
        const configDir = path.join(HOME_DIR_PATH, 'ssl', config.getName());

        fs.mkdirSync(configDir, { recursive: true });

        const crtFile = path.join(configDir, 'bundle.crt');

        fs.writeFileSync(crtFile, ctx.certificateFile, 'utf8');

        if (ctx.privateKeyFile) {
          const keyFile = path.join(configDir, 'private.key');

          fs.writeFileSync(keyFile, ctx.privateKeyFile, 'utf8');
        }

        config.set('platform.dapi.envoy.ssl.enabled', true);
      },
    }]);
}

module.exports = saveCertificateTask;
