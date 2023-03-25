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
                name: 'chainFile',
                message: 'Path to certificate chain file',
                validate: validateFileExists,
              },
              {
                name: 'privateFile',
                message: 'Path to certificate key file',
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
    };

    return new Listr([
      {
        title: 'Configure SSL certificate',
        task: async (ctx, task) => {
          // TODO Move to upper function to show output in upper task?
          const choices = [
            { name: SSL_PROVIDERS.ZEROSSL, message: 'ZeroSSL' },
            { name: SSL_PROVIDERS.FILE, message: 'File on disk' },
          ];

          if (ctx.preset !== PRESET_MAINNET) {
            choices.push({ name: SSL_PROVIDERS.SELF_SIGNED, message: 'Self-signed' });
          }

          ctx.certificateProvider = await task.prompt({
            type: 'select',
            header: `  High-performance masternodes are required use TLS encryption on the DAPI
  endpoint through which they service the network. This encryption is achieved
  by loading an SSL certificate signed against the IP address specified in the
  registration transaction. The certificate should be recognized by common web
  browsers, and must therefore be issued by a well-known Certificate Authority
  (CA). Dashmate offers three options to configure this certificate:

    ZeroSSL      - Provide a (free) ZeroSSL API key and let dashmate configure the certificate
    File on disk - Provide your own certificate to dashmate
    Self-signed  - Generate your own self-signed certificate (testnet only)\n`,
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
