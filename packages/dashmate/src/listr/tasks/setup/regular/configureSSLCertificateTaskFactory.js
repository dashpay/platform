const fs = require('fs');

const { Listr } = require('listr2');

const {
  PRESET_MAINNET,
  SSL_PROVIDERS,
} = require('../../../../constants');

const listCertificates = require('../../../../ssl/zerossl/listCertificates');

const validateFileExists = require('../../../prompts/validators/validateFileExists');

/**
 * @returns {configureSSLCertificateTask}
 */
function configureSSLCertificateTaskFactory(
  saveCertificateTask,
  obtainZeroSSLCertificateTask,
) {
  /**
   * @typedef configureSSLCertificateTask
   * @returns {Listr}
   */
  function configureSSLCertificateTask() {
    return new Listr([
      {
        task: async (ctx, task) => {
          const choices = [
            { name: SSL_PROVIDERS.ZEROSSL, message: 'ZeroSSL' },
            { name: SSL_PROVIDERS.FILE, message: 'File on disk' },
          ];

          if (ctx.preset !== PRESET_MAINNET) {
            choices.push({ name: SSL_PROVIDERS.SELF_SIGNED, message: 'Self-signed' });
          }

          ctx.certificateProvider = await task.prompt({
            type: 'select',
            message: 'How do you prefer to configure SSL',
            choices,
            initial: SSL_PROVIDERS.ZEROSSL,
          });

          ctx.config.set('platform.dapi.envoy.ssl.provider', ctx.certificateProvider);
        },
      },
      {
        title: 'Obtain ZeroSSL certificate',
        enabled: (ctx) => ctx.certificateProvider === SSL_PROVIDERS.ZEROSSL,
        task: async (ctx, task) => {
          const apiKey = await task.prompt({
            type: 'input',
            message: 'Enter ZeroSSL API key',
            validate: async (state) => {
              try {
                await listCertificates(state);

                return true;
              } catch (e) {
                // do nothing
              }

              return 'Please enter a valid ZeroSSL API key';
            },
          });

          ctx.config.set('platform.dapi.envoy.ssl.providerConfigs.zerossl.apiKey', apiKey);

          return obtainZeroSSLCertificateTask(ctx.config);
        },
      },
      {
        title: 'Set SSL certificate file',
        enabled: (ctx) => ctx.certificateProvider === SSL_PROVIDERS.FILE,
        task: async (ctx, task) => {

          const form = await task.prompt({
            type: 'form',
            message: 'Provide paths to your certificate files',
            choices: [
              {
                name: 'chainFile',
                message: 'Path to certificate chain file',
                validate: validateFileExists,
              },
              {
                name: 'privateFile',
                message: 'Path to private file',
                validate: validateFileExists,
              },
            ],
            validate: ({ chainFile, privateFile }) => validateFileExists(chainFile)
              && validateFileExists(privateFile),
          });

          ctx.certificate = fs.readFileSync(form.chainFile, 'utf8');
          ctx.keyPair = {
            privateKey: fs.readFileSync(form.privateFile, 'utf8'),
          };

          return saveCertificateTask(ctx.config);
        },
      },
    ]);
  }

  return configureSSLCertificateTask;
}

module.exports = configureSSLCertificateTaskFactory;
