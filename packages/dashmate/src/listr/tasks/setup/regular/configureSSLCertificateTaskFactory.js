import { Listr } from 'listr2';

import {
  SSL_PROVIDERS,
  NODE_TYPE_FULLNODE,
} from '../../../../constants.js';

import listCertificates from '../../../../ssl/zerossl/listCertificates.js';
import promptOrThrow from '../../../../util/promptOrThrow.js';

/**
 * The certificate providers a node of this type may be set up with
 *
 * A self-signed certificate is not publicly trusted, and `update` refuses one
 * on any masternode, mainnet or testnet, so only fullnodes are offered it.
 *
 * @param {string} nodeType
 * @return {string[]}
 */
export function getCertificateProviders(nodeType) {
  const providers = [SSL_PROVIDERS.ZEROSSL, SSL_PROVIDERS.LETSENCRYPT, SSL_PROVIDERS.FILE];

  if (nodeType === NODE_TYPE_FULLNODE) {
    providers.push(SSL_PROVIDERS.SELF_SIGNED);
  }

  return providers;
}

/**
 * @param {string} apiKey
 * @return {Promise<boolean>}
 */
async function isZeroSslApiKeyAccepted(apiKey) {
  try {
    await listCertificates(apiKey);

    return true;
  } catch {
    return false;
  }
}

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
   * Choose how the gateway gets its certificate, and get it
   *
   * Answers may come from the context (ctx.certificateProvider,
   * ctx.zeroSslApiKey, ctx.fileCertificateProviderForm) instead of prompts.
   * Whether it may prompt is ctx.interactive, which the caller states.
   *
   * @typedef configureSSLCertificateTask
   * @returns {Listr}
   */
  function configureSSLCertificateTask() {
    const providerTasks = {
      [SSL_PROVIDERS.FILE]: {
        title: 'Set SSL certificate file',
        enabled: (ctx) => ctx.certificateProvider === SSL_PROVIDERS.FILE,
        task: async (ctx) => installCertificateFilesTask(ctx.config, {
          interactive: ctx.interactive,
        }),
      },
      [SSL_PROVIDERS.ZEROSSL]: {
        title: 'Obtain ZeroSSL certificate',
        task: async (ctx, task) => {
          let apiKey = ctx.zeroSslApiKey;

          if (apiKey === undefined) {
            apiKey = await promptOrThrow(task, {
              type: 'password',
              message: 'Enter ZeroSSL API key',
              validate: async (key) => (await isZeroSslApiKeyAccepted(key))
                || 'Please enter a valid ZeroSSL API key',
            }, { interactive: ctx.interactive });
          } else if (!(await isZeroSslApiKeyAccepted(apiKey))) {
            throw new Error('ZeroSSL did not accept the API key');
          }

          ctx.config.set('platform.gateway.ssl.providerConfigs.zerossl.apiKey', apiKey);

          return obtainZeroSSLCertificateTask(ctx.config, {
            onCertificateCreated: () => {
              configFile.setConfig(ctx.config);
              configFileRepository.write(configFile);

              // A failure from here on leaves the answers saved.
              ctx.isZeroSslCertificateRecorded = true;
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

    const choiceMessages = {
      [SSL_PROVIDERS.ZEROSSL]: 'ZeroSSL',
      [SSL_PROVIDERS.LETSENCRYPT]: "Let's Encrypt",
      [SSL_PROVIDERS.FILE]: 'File on disk',
      [SSL_PROVIDERS.SELF_SIGNED]: 'Self-signed',
    };

    return new Listr([
      {
        title: 'Configure SSL certificate',
        task: async (ctx, task) => {
          const providers = getCertificateProviders(ctx.nodeType);

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

          if (providers.includes(SSL_PROVIDERS.SELF_SIGNED)) {
            header += '    Self-signed    - Generate your own self-signed certificate\n';
          }

          if (!ctx.certificateProvider) {
            ctx.certificateProvider = await promptOrThrow(task, {
              type: 'select',
              header,
              message: 'How do you want to configure SSL?',
              choices: providers.map((name) => ({ name, message: choiceMessages[name] })),
              initial: SSL_PROVIDERS.ZEROSSL,
            }, { interactive: ctx.interactive });
          }

          // An answer given in advance is held to the same rule as the prompt.
          if (!providers.includes(ctx.certificateProvider)) {
            throw new Error(`The ${ctx.certificateProvider} certificate provider cannot be used for`
              + ` a ${ctx.nodeType}. Choose one of: ${providers.join(', ')}`);
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
