const { Listr } = require('listr2');
const path = require('path');
const fs = require('fs');
const { HOME_DIR_PATH } = require('../../../constants');

/**
 *
 * @param {Config} config
 * @return {Promise<Listr>}
 */
async function setupCertificateTask(config) {
  return new Listr([
    {
      title: 'Setup verification server',
      task: async (ctx) => {
        const configDir = path.join(HOME_DIR_PATH, 'ssl', config.getName());

        fs.mkdirSync(configDir, { recursive: true });

        const crtFile = path.join(configDir, 'bundle.crt');
        const keyFile = path.join(configDir, 'private.key');

        fs.writeFileSync(crtFile, ctx.certificate, 'utf8');
        fs.writeFileSync(keyFile, ctx.csr, 'utf8');

        config.set('platform.dapi.envoy.ssl.enabled', true);
      },
    }]);
}

module.exports = setupCertificateTask;
