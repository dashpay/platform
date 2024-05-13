import fs from 'fs';
import { Listr } from 'listr2';

import {
  PRESET_MAINNET,
  SSL_PROVIDERS,
  NODE_TYPE_FULLNODE,
} from '../../../../constants.js';
import validateFileExists from '../../../prompts/validators/validateFileExists.js';
import listCertificates from '../../../../ssl/zerossl/listCertificates.js';

/**
 * @param {saveCertificateTask} saveCertificateTask
 * @param {obtainZeroSSLCertificateTask} obtainZeroSSLCertificateTask
 * @param {obtainSelfSignedCertificateTask} obtainSelfSignedCertificateTask
 * @returns {configureSSLCertificateTask}
 */
export default function configureSSLCertificateTaskFactory(
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
          let form = ctx.fileCertificateProviderForm;

          if (!ctx.fileCertificateProviderForm) {
            form = await task.prompt({
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
              validate: ({ chainFilePath, privateFilePath }) => {
                if (!validateFileExists(chainFilePath)) {
                  return 'certificate chain file path is not valid';
                }

                if (!validateFileExists(privateFilePath)) {
                  return 'certificate key file path is not valid';
                }

                if (chainFilePath === privateFilePath) {
                  return 'the same path for both files';
                }

                return true;
              },
            });
          }

          ctx.certificateFile = fs.readFileSync(form.chainFilePath, 'utf8');
          ctx.privateKeyFile = fs.readFileSync(form.privateFilePath, 'utf8');

          return saveCertificateTask(ctx.config);
        },
      },
      [SSL_PROVIDERS.ZEROSSL]: {
        title: 'Obtain ZeroSSL certificate',
        task: async (ctx, task) => {
          const apiKey = await task.prompt({
            type: 'input',
            message: 'Enter ZeroSSL API key',
            validate: async (key) => {
              try {
                await listCertificates(key);

                return true;
              } catch (e) {
                // do nothing
              }

              return 'Please enter a valid ZeroSSL API key';
            },
          });

          ctx.config.set('platform.gateway.ssl.providerConfigs.zerossl.apiKey', apiKey);

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

          let header = `  Evonodes are required use TLS encryption on the DAPI
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

          if (!ctx.certificateProvider) {
            ctx.certificateProvider = await task.prompt({
              type: 'select',
              header,
              message: 'How do you want to configure SSL?',
              choices,
              initial: SSL_PROVIDERS.ZEROSSL,
            });
          }

          ctx.config.set('platform.gateway.ssl.provider', ctx.certificateProvider);

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
