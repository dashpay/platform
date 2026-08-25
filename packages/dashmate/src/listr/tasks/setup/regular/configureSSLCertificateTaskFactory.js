import { Listr } from 'listr2';

import {
  PRESET_MAINNET,
  SSL_PROVIDERS,
  NODE_TYPE_FULLNODE,
} from '../../../../constants.js';

import listCertificates from '../../../../ssl/zerossl/listCertificates.js';

/**
 * @param {installCertificateFilesTask} installCertificateFilesTask
 * @param {obtainZeroSSLCertificateTask} obtainZeroSSLCertificateTask
 * @param {obtainSelfSignedCertificateTask} obtainSelfSignedCertificateTask
 * @param {obtainLetsEncryptCertificateTask} obtainLetsEncryptCertificateTask
 * @param {ConfigFile} configFile
 * @param {ConfigFileJsonRepository} configFileRepository
 * @returns {configureSSLCertificateTask}
 */
export default function configureSSLCertificateTaskFactory(
  installCertificateFilesTask,
  obtainZeroSSLCertificateTask,
  obtainSelfSignedCertificateTask,
  obtainLetsEncryptCertificateTask,
  configFile,
  configFileRepository,
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
        task: async (ctx) => installCertificateFilesTask(ctx.config, { interactive: true }),
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

          return obtainZeroSSLCertificateTask(ctx.config, {
            onCertificateCreated: () => {
              configFile.setConfig(ctx.config);
              configFileRepository.write(configFile);
            },
          });
        },
      },
      [SSL_PROVIDERS.SELF_SIGNED]: {
        title: 'Generate self-signed certificate',
        task: async (ctx) => obtainSelfSignedCertificateTask(ctx.config),
      },
      [SSL_PROVIDERS.LETSENCRYPT]: {
        title: 'Obtain Let\'s Encrypt certificate',
        // No contact address is asked for. It is optional under RFC 8555,
        // Let's Encrypt ended expiry notifications in 2025 and does not keep an
        // address supplied through ACME, so the question bought nothing.
        task: async (ctx) => obtainLetsEncryptCertificateTask(ctx.config),
      },
    };

    return new Listr([
      {
        title: 'Configure SSL certificate',
        task: async (ctx, task) => {
          // Setup asks the operator a question at every step, so it cannot run
          // unattended and states this rather than detecting it. The obtain
          // tasks read it to decide whether they may prompt.
          ctx.interactive = true;

          const choices = [
            { name: SSL_PROVIDERS.ZEROSSL, message: 'ZeroSSL' },
            { name: SSL_PROVIDERS.LETSENCRYPT, message: "Let's Encrypt" },
            { name: SSL_PROVIDERS.FILE, message: 'File on disk' },
          ];

          const isSelfSignedEnabled = ctx.preset !== PRESET_MAINNET
            || ctx.nodeType === NODE_TYPE_FULLNODE;

          let header = `  Evonodes are required use TLS encryption on the DAPI
  endpoint through which they service the network. This encryption is achieved
  by loading an SSL certificate signed against the IP address specified in the
  registration transaction. The certificate should be recognized by common web
  browsers, and must therefore be issued by a well-known Certificate Authority
  (CA). Dashmate offers several options to configure this certificate:

    ZeroSSL        - Provide a ZeroSSL API key and let dashmate configure the certificate
                     https://zerossl.com/documentation/api/ ("Access key" section)
    Let's Encrypt  - Free certificates for your IP address, no account needed
    File on disk   - Provide your own certificate to dashmate\n`;

          if (isSelfSignedEnabled) {
            header += '    Self-signed    - Generate your own self-signed certificate\n';

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
