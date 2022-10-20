const { Listr } = require('listr2');
const fs = require('fs');

const publicIp = require('public-ip');

const BlsSignatures = require('@dashevo/bls');

const { PrivateKey } = require('@dashevo/dashcore-lib');

const {
  SSL_PROVIDERS,
  NODE_TYPES,
  NODE_TYPE_MASTERNODE,
  PRESET_MAINNET,
} = require('../../../constants');

/**
 * @param {ConfigFile} configFile
 * @param {generateBlsKeys} generateBlsKeys
 * @param {tenderdashInitTask} tenderdashInitTask
 * @param {registerMasternodeTask} registerMasternodeTask
 * @param {renderServiceTemplates} renderServiceTemplates
 * @param {writeServiceConfigs} writeServiceConfigs
 * @param {obtainZeroSSLCertificateTask} obtainZeroSSLCertificateTask
 * @param {saveCertificateTask} saveCertificateTask
 * @param {listCertificates} listCertificates
 */
function setupRegularPresetTaskFactory(
  configFile,
  generateBlsKeys,
  tenderdashInitTask,
  registerMasternodeTask,
  renderServiceTemplates,
  writeServiceConfigs,
  obtainZeroSSLCertificateTask,
  saveCertificateTask,
  listCertificates,
) {
  /**
   * @typedef {setupRegularPresetTask}
   * @return {Listr}
   */
  function setupRegularPresetTask() {
    return new Listr([
      {
        task: (ctx) => {
          ctx.config = configFile.getConfig(ctx.preset);
        },
      },
      {
        title: 'Set node type',
        task: async (ctx, task) => {
          if (ctx.nodeType === undefined) {
            ctx.nodeType = await task.prompt([
              {
                type: 'select',
                message: 'Select node type',
                choices: NODE_TYPES,
                initial: NODE_TYPE_MASTERNODE,
              },
            ]);
          }

          ctx.config.set('core.masternode.enable', ctx.nodeType === NODE_TYPE_MASTERNODE);

          // eslint-disable-next-line no-param-reassign
          task.output = `Selected ${ctx.nodeType} type\n`;
        },
        options: { persistentOutput: true },
      },
      {
        title: 'Configure external IP address',
        task: async (ctx, task) => {
          if (ctx.externalIp === undefined) {
            ctx.externalIp = await task.prompt([
              {
                type: 'input',
                message: 'Enter node public IP (Enter to accept detected IP)',
                initial: () => publicIp.v4(),
              },
            ]);
          }

          ctx.config.set('externalIp', ctx.externalIp);

          // eslint-disable-next-line no-param-reassign
          task.output = `${ctx.externalIp} is set\n`;
        },
        options: { persistentOutput: true },
      },
      {
        title: 'Set masternode operator private key',
        enabled: (ctx) => ctx.nodeType === NODE_TYPE_MASTERNODE,
        task: async (ctx, task) => {
          if (ctx.operatorBlsPrivateKey === undefined) {
            const { privateKey: generatedPrivateKeyHex } = await generateBlsKeys();

            ctx.operatorBlsPrivateKey = await task.prompt([
              {
                type: 'input',
                message: 'Enter operator BLS private key (Enter to accept generated key)',
                initial: generatedPrivateKeyHex,
              },
            ]);
          }

          const operatorBlsPrivateKeyBuffer = Buffer.from(ctx.operatorBlsPrivateKey, 'hex');

          const blsSignatures = await BlsSignatures();
          const { PrivateKey: BlsPrivateKey, BasicSchemeMPL } = blsSignatures;

          const privateKey = BlsPrivateKey.from_bytes(operatorBlsPrivateKeyBuffer, true);
          const publicKey = BasicSchemeMPL.sk_to_g1(privateKey);
          const publicKeyHex = Buffer.from(publicKey.serialize()).toString('hex');

          ctx.config.set('core.masternode.operator.privateKey', ctx.operatorBlsPrivateKey);

          ctx.operator = {
            publicKey: publicKeyHex,
          };

          // eslint-disable-next-line no-param-reassign
          task.output = `BLS public key: ${publicKeyHex}\nBLS private key: ${ctx.operatorBlsPrivateKey}`;
        },
        options: { persistentOutput: true },
      },
      {
        title: 'Register masternode',
        enabled: (ctx) => (
          ctx.nodeType === NODE_TYPE_MASTERNODE
          && ctx.fundingPrivateKeyString !== undefined
        ),
        task: (ctx) => {
          if (ctx.preset === PRESET_MAINNET) {
            throw new Error('For your own security, this tool will not process mainnet private keys. You should consider the private key you entered to be compromised.');
          }

          const fundingPrivateKey = new PrivateKey(ctx.fundingPrivateKeyString, ctx.preset);
          ctx.fundingAddress = fundingPrivateKey.toAddress(ctx.preset).toString();

          // Write configs
          const configFiles = renderServiceTemplates(ctx.config);
          writeServiceConfigs(ctx.config.getName(), configFiles);

          return registerMasternodeTask(ctx.config);
        },
        options: { persistentOutput: true },
      },
      {
        title: 'Initialize Tenderdash',
        enabled: (ctx) => ctx.preset !== PRESET_MAINNET,
        task: (ctx) => tenderdashInitTask(ctx.config),
      },
      {
        title: 'Set default config',
        task: (ctx, task) => {
          configFile.setDefaultConfigName(ctx.preset);

          // eslint-disable-next-line no-param-reassign
          task.output = `${ctx.config.getName()} set as default config\n`;
        },
      },
      {
        title: 'Set SSL certificate',
        task: async (ctx, task) => {
          const sslProviders = [...SSL_PROVIDERS].filter((item) => item !== 'selfSigned');

          ctx.certificateProvider = await task.prompt({
            type: 'select',
            message: 'Select SSL certificate provider',
            choices: sslProviders,
            initial: sslProviders[0],
          });

          ctx.config.set('platform.dapi.envoy.ssl.provider', ctx.certificateProvider);
        },
      },
      {
        title: 'Obtain ZeroSSL certificate',
        enabled: (ctx) => ctx.certificateProvider === 'zerossl',
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
        title: 'Set SSL certificate',
        enabled: (ctx) => ctx.certificateProvider === 'manual',
        task: async (ctx, task) => {
          const bundleFilePath = await task.prompt({
            type: 'input',
            message: 'Enter the path to your certificate chain file',
            validate: (state) => {
              if (fs.existsSync(state)) {
                return true;
              }

              return 'Please enter a valid path to your certificate chain file';
            },
          });

          const privateKeyFilePath = await task.prompt({
            type: 'input',
            message: 'Enter the path to your private key file',
            validate: (state) => {
              if (fs.existsSync(state)) {
                return true;
              }

              return 'Please enter a valid path to your private key file';
            },
          });

          ctx.certificate = fs.readFileSync(bundleFilePath, 'utf8');
          ctx.keyPair = {
            privateKey: fs.readFileSync(privateKeyFilePath, 'utf8'),
          };

          return saveCertificateTask(ctx.config);
        },
      },
    ]);
  }

  return setupRegularPresetTask;
}

module.exports = setupRegularPresetTaskFactory;
