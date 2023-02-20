const { Listr } = require('listr2');
const fs = require('fs');

const publicIp = require('public-ip');

const BlsSignatures = require('@dashevo/bls');

const { PrivateKey } = require('@dashevo/dashcore-lib');

const crypto = require('crypto');

const {
  SSL_PROVIDERS,
  NODE_TYPES,
  NODE_TYPE_MASTERNODE,
  PRESET_MAINNET,
  NODE_TYPE_HPMN,
  NODE_TYPE_FULLNODE,
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
        title: 'Node type',
        task: async (ctx, task) => {
          if (ctx.nodeType === undefined) {
            ctx.nodeType = await task.prompt([
              {
                type: 'select',
                header: '  Dash network has different node types\n  Blue\n  Green\n  Red\n  We'
                  + ' should'
                  + ' explain their purpose and costs\n',
                message: 'Select node type',
                choices: [
                  { name: NODE_TYPE_MASTERNODE },
                  { name: NODE_TYPE_HPMN, message: 'high-performance masternode' },
                  { name: NODE_TYPE_FULLNODE },
                ],
                initial: NODE_TYPE_HPMN,
              },
            ]);

            // eslint-disable-next-line no-param-reassign
            task.output = ctx.nodeType;
          }
        },
        options: {
          persistentOutput: true,
        },
      },
      {
        enabled: (ctx) => ctx.nodeType === NODE_TYPE_MASTERNODE || ctx.nodeType === NODE_TYPE_HPMN,
        task: async (ctx, task) => {
          ctx.isMasternodeRegistered = await task.prompt([
            {
              type: 'toggle',
              header: 'Tell what it means and what we gonna do in both cases\n',
              message: 'Is your masternode already registered?',
              enabled: 'Yep',
              disabled: 'Nope',
            },
          ]);

          ctx.config.set('core.masternode.enable', true);
        },
      },
      {
        title: 'Masternode operator key',
        enabled: (ctx) => ctx.isMasternodeRegistered,
        task: async (ctx, task) => {
          const blsSignatures = await BlsSignatures();
          const { PrivateKey: BlsPrivateKey } = blsSignatures;

          function validate(value) {
            if (value.length < 1) {
              return 'should not be empty';
            }

            const operatorBlsPrivateKeyBuffer = Buffer.from(value, 'hex');

            let key;
            try {
              key = BlsPrivateKey.fromBytes(operatorBlsPrivateKeyBuffer, true);
            } catch (e) {
              return 'invalid key';
            } finally {
              if (key) {
                key.delete();
              }
            }

            return true;
          }

          if (ctx.operatorBlsPrivateKey === undefined) {
            ctx.operatorBlsPrivateKey = await task.prompt([
              {
                type: 'input',
                header: 'Masternode operator BLS private key... \n'
                  + 'you can take it there and put here. Must be HEX.\n',
                message: 'Enter BLS private key',
                validate,
              },
            ]);
          } else {
            const result = validate(ctx.operatorBlsPrivateKey);

            if (result !== true) {
              throw new Error(`operator private key: ${result}`);
            }
          }

          ctx.config.set('core.masternode.operator.privateKey', ctx.operatorBlsPrivateKey);

          // eslint-disable-next-line no-param-reassign
          task.output = '*******************************************';
        },
        options: {
          persistentOutput: true,
        },
      },
      {
        title: 'Platform P2P Key',
        enabled: (ctx) => ctx.isMasternodeRegistered && ctx.nodeType === NODE_TYPE_HPMN,
        task: async (ctx, task) => {
          // TODO: Do we accept HEX or base64?

          function validate(value) {
            if (value.length < 1) {
              return 'should not be empty';
            }

            // TODO: Implement validation
            // const privateKeyDer = Buffer.concat([
            //   Buffer.from('302a300506032b6570032100', 'hex'), // Static value
            //   Buffer.from(value, 'hex'),
            // ]);
            //
            // const verifyKey = crypto.createPrivateKey({
            //   format: 'der',
            //   type: 'pkcs8',
            //   privateKeyDer,
            // });
            //

            return true;
          }

          if (ctx.platformP2PKey === undefined) {
            ctx.platformP2PKey = await task.prompt([
              {
                type: 'input',
                header: 'Platform P2P private key ... we accept base64 or hex?...',
                message: 'Enter ED25519 private key',
                validate,
              },
            ]);
          } else {
            const result = validate(ctx.platformP2PKey);

            if (result !== true) {
              throw new Error(`platform p2p key: ${result}`);
            }
          }

          // TODO: Derive node id from key
          // config.set('platform.drive.tenderdash.nodeId', nodeId);

          ctx.config.set('platform.drive.tenderdash.nodeKey', ctx.platformP2PKey);
        },
        options: {
          persistentOutput: true,
        },
      },
      {
        title: 'Masternode keys',
        enabled: (ctx) => !ctx.isMasternodeRegistered
          && (ctx.nodeType === NODE_TYPE_HPMN || ctx.nodeType === NODE_TYPE_MASTERNODE),
        task: async (ctx, task) => {
          ctx.masternodeOwnerKeys = await task.prompt([
            {
              type: 'form',
              header: 'The user should use a secured wallet to generate a key, and provide the'
                + ' resulting public keys. (deploy tool example)\n',
              message: 'Please provide the following information:',
              choices: [
                { name: 'owner', message: 'Owner public key' },
                { name: 'voting', message: 'Voting public key' },
                { name: 'payout', message: 'Payout script' },
              ],
            },
          ]);
        },
      },
      {
        title: 'External IP address',
        task: async (ctx, task) => {
          if (ctx.externalIp === undefined) {
            ctx.externalIp = await task.prompt([
              {
                type: 'input',
                header: 'The node external IP address must be static and will be used by the'
                  + ' network ..',
                message: 'Enter host public IP',
                initial: () => publicIp.v4(),
              },
            ]);
          }

          ctx.config.set('externalIp', ctx.externalIp);

          // eslint-disable-next-line no-param-reassign
          task.output = ctx.externalIp;
        },
        options: {
          persistentOutput: true,
        },
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
          const { PrivateKey: BlsPrivateKey } = blsSignatures;

          const privateKey = BlsPrivateKey.fromBytes(operatorBlsPrivateKeyBuffer, true);
          const publicKey = privateKey.getG1();
          const publicKeyHex = Buffer.from(publicKey.serialize()).toString('hex');

          ctx.config.set('core.masternode.operator.privateKey', ctx.operatorBlsPrivateKey);

          ctx.operator = {
            publicKey: publicKeyHex,
          };

          privateKey.delete();
          publicKey.delete();

          // eslint-disable-next-line no-param-reassign
          task.output = `BLS public key: ${publicKeyHex}\nBLS private key: ${ctx.operatorBlsPrivateKey}`;
        },
        options: { persistentOutput: true },
      },
      {
        title: 'SSL certificate',
        enabled: (ctx) => !ctx.certificateProvider,
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
          let apiKey = ctx.zeroSslApiKey;

          if (!apiKey) {
            apiKey = await task.prompt({
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
          }

          ctx.config.set('platform.dapi.envoy.ssl.providerConfigs.zerossl.apiKey', apiKey);

          return obtainZeroSSLCertificateTask(ctx.config);
        },
      },
      {
        title: 'Set SSL certificate',
        enabled: (ctx) => ctx.certificateProvider === 'manual',
        task: async (ctx, task) => {
          if (!ctx.sslCertificateFilePath) {
            ctx.sslCertificateFilePath = await task.prompt({
              type: 'input',
              message: 'Enter the path to your certificate chain file',
              validate: (state) => {
                if (fs.existsSync(state)) {
                  return true;
                }

                return 'Please enter a valid path to your certificate chain file';
              },
            });
          }

          if (!ctx.sslCertificatePrivateKeyFilePath) {
            ctx.sslCertificatePrivateKeyFilePath = await task.prompt({
              type: 'input',
              message: 'Enter the path to your private key file',
              validate: (state) => {
                if (fs.existsSync(state)) {
                  return true;
                }

                return 'Please enter a valid path to your private key file';
              },
            });
          }

          ctx.certificate = fs.readFileSync(ctx.sslCertificateFilePath, 'utf8');
          ctx.keyPair = {
            privateKey: fs.readFileSync(ctx.sslCertificatePrivateKeyFilePath, 'utf8'),
          };

          return saveCertificateTask(ctx.config);
        },
      },
      {
        task: (ctx) => {
          configFile.setDefaultConfigName(ctx.preset);
        },
      },
    ]);
  }

  return setupRegularPresetTask;
}

module.exports = setupRegularPresetTaskFactory;
