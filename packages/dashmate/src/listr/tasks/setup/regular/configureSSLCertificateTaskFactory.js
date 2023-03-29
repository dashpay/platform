const fs = require('fs');

const { Listr } = require('listr2');

const {
  PRESET_MAINNET,
  SSL_PROVIDERS,
  NODE_TYPE_FULLNODE,
} = require('../../../../constants');

const listCertificates = require('../../../../ssl/zerossl/listCertificates');

const validateFileExists = require('../../../prompts/validators/validateFileExists');

/**
 * @param {saveCertificateTask} saveCertificateTask
 * @param {obtainZeroSSLCertificateTask} obtainZeroSSLCertificateTask
 * @param {obtainSelfSignedCertificateTask} obtainSelfSignedCertificateTask
 * @returns {configureSSLCertificateTask}
 */
function configureSSLCertificateTaskFactory(
  saveCertificateTask,
  obtainZeroSSLCertificateTask,
  obtainSelfSignedCertificateTask,
) {
  /**
   * @typedef configureSSLCertificateTask
   * @returns {Listr}
   */
  function configureSSLCertificateTask() {
    const providerTasks = {
      [SSL_PROVIDERS.FILE]: {
        title: 'Set SSL certificate file',
        enabled: (ctx) => ctx.certificateProvider === SSL_PROVIDERS.FILE,
        task: async (ctx, task) => {
          const form = await task.prompt({
            type: 'form',
            message: 'Specify paths to your certificate files',
            choices: [
              {
                name: 'chainFilePath',
                message: 'Path to certificate chain file',
                validate: validateFileExists,
              },
              {
                name: 'privateFilePath',
                message: 'Path to certificate key file',
                validate: validateFileExists,
              },
            ],
            validate: ({ chainFilePath, privateFilePath }) => () => {
              if (!validateFileExists(chainFilePath) || !validateFileExists(privateFilePath)) {
                return false;
              }

              if (chainFilePath === privateFilePath) {
                return 'the same path for both files';
              }

              return true;
            },
          });

          ctx.certificate = fs.readFileSync(form.chainFilePath, 'utf8');
          ctx.keyPair = {
            privateKey: fs.readFileSync(form.privateFilePath, 'utf8'),
          };

          return saveCertificateTask(ctx.config);
        },
      },
      [SSL_PROVIDERS.ZEROSSL]: {
        title: 'Obtain ZeroSSL certificate',
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
      [SSL_PROVIDERS.SELF_SIGNED]: {
        title: 'Generate self-signed certificate',
        task: async (ctx) => obtainSelfSignedCertificateTask(ctx.config),
      },
    };

    return new Listr([
      {
        title: 'Configure SSL certificate',
        task: async (ctx, task) => {
          const choices = [
            { name: SSL_PROVIDERS.ZEROSSL, message: 'ZeroSSL' },
            { name: SSL_PROVIDERS.FILE, message: 'File on disk' },
          ];

          const isSelfSignedEnabled = ctx.preset !== PRESET_MAINNET
            || ctx.nodeType === NODE_TYPE_FULLNODE;

          let header = `  High-performance masternodes are required use TLS encryption on the DAPI
  endpoint through which they service the network. This encryption is achieved
  by loading an SSL certificate signed against the IP address specified in the
  registration transaction. The certificate should be recognized by common web
  browsers, and must therefore be issued by a well-known Certificate Authority
  (CA). Dashmate offers three options to configure this certificate:

    ZeroSSL      - Provide a ZeroSSL API key and let dashmate configure the certificate
                   https://zerossl.com/documentation/api/ ("Access key" section)
    File on disk - Provide your own certificate to dashmate\n`;

          if (isSelfSignedEnabled) {
            header += '    Self-signed  - Generate your own self-signed certificate\n';

            choices.push({ name: SSL_PROVIDERS.SELF_SIGNED, message: 'Self-signed' });
          }

          ctx.certificateProvider = await task.prompt({
            type: 'select',
            header,
            message: 'How do you want to configure SSL?',
            choices,
            initial: SSL_PROVIDERS.ZEROSSL,
          });

          ctx.config.set('platform.dapi.envoy.ssl.provider', ctx.certificateProvider);

          // eslint-disable-next-line no-param-reassign
          task.output = ctx.certificateProvider;

          return new Listr([providerTasks[ctx.certificateProvider]]);
        },
        options: {
          persistentOutput: true,
          collapse: true,
        },
      },
    ]);
  }

  return configureSSLCertificateTask;
}

module.exports = configureSSLCertificateTaskFactory;
