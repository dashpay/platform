const { Listr } = require('listr2');
const fs = require('fs');

const publicIp = require('public-ip');

const BlsSignatures = require('@dashevo/bls');

const { PrivateKey, PublicKey, Address } = require('@dashevo/dashcore-lib');

const crypto = require('crypto');

const placeholder = require('enquirer/lib/placeholder');

const {
  SSL_PROVIDERS,
  NODE_TYPES,
  NODE_TYPE_MASTERNODE,
  PRESET_MAINNET,
  NODE_TYPE_HPMN,
  NODE_TYPE_FULLNODE,
} = require('../../../constants');
const { base } = require('../../../../configs/system');

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
                header: '  The Dash network consists of several different node types:'
                  + ' \n    Full nodes: Host a full copy of the Dash blockchain (no collateral required)'
                  + ' \n    Masternodes: Full node features, plus Core services such as ChainLocks and InstantSend (1000 DASH collateral)'
                  + ' \n    High-performance masternodes: Masternode features, plus Platform services such as DAPI and Drive (4000 DASH collateral)\n',
                message: 'Select node type',
                choices: [
                  { name: NODE_TYPE_MASTERNODE, hint: '1000 DASH collateral' },
                  { name: NODE_TYPE_HPMN, message: 'high-performance masternode', hint: '4000 DASH collateral' },
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
          let header;
          if (ctx.nodeType === NODE_TYPE_HPMN) {
            header = 'If your HP masternode is already registered, we will import your masternode'
              + ' operator and platform node keys to configure an HP masternode.'
              + ' Please make sure your IP address has not changed, otherwise you will need'
              + ' to create a provider update service transaction.\n\n'
              + ' If you are registering a new HP masternode, I will provide more information'
              + ' and help you to generate the necessary keys.\n';
          } else {
            header = 'If your masternode is already registered, we will import your masternode'
              + ' operator key to configure a masternode.'
              + ' Please make sure your IP address has not changed, otherwise you will need'
              + ' to create a provider update service transaction.\n\n'
              + ' If you are registering a new masternode, I will provide more information'
              + ' and help you to generate the necessary keys.\n';
          }

          ctx.isMasternodeRegistered = await task.prompt([
            {
              type: 'toggle',
              header,
              message: 'Is your masternode already registered?',
              enabled: 'Yes',
              disabled: 'No',
            },
          ]);

          ctx.config.set('core.masternode.enable', true);
        },
      },
      {
        enabled: (ctx) => (ctx.nodeType === NODE_TYPE_MASTERNODE || ctx.nodeType === NODE_TYPE_HPMN)
          && !ctx.isMasternodeRegistered,
        task: async (ctx, task) => {
          ctx.registrar = await task.prompt([
            {
              type: 'select',
              header: 'For security reasons, Dash masternodes should never store masternode owner'
                + ' or collateral private keys. Dashmate therefore cannot register a masternode for'
                + ' you directly. Instead, we will generate RPC commands that you can use in Dash'
                + ' Core or other external tools where the keys are handled securely. During this'
                + ' process, dashmate can optionally generate configuration elements as necessary,'
                + ' such as certificates, the BLS operator key and the node id, since this is the'
                + ' only information necessary for dashmate to configure the masternode.',
              message: 'Which tool will you use to register your masternode?',
              choices: [
                { name: 'core', message: 'Dash Core (Wallet?)' },
                { name: 'other', message: 'Other' },
              ],
              initial: 'core',
            },
          ]);
        },
      },
      {
        title: 'Register masternode with Dash Core',
        enabled: (ctx) => ctx.registrar === 'core'
          && (ctx.nodeType === NODE_TYPE_HPMN || ctx.nodeType === NODE_TYPE_MASTERNODE),
        task: async (ctx, task) => {
          function validateOutputIndex(value) {
            const index = Math.floor(Number(value));

            return index >= 0 && index.toString() === value;
          }

          function validateTxHash(value) {
            return value.length === 64;
          }

          function validateECDSAPublicKey(value) {
            try {
              PublicKey(value);

              return true;
            } catch (e) {
              return false;
            }
          }

          function validateAddress(value) {
            try {
              Address(value);

              return true;
            } catch (e) {
              return false;
            }
          }

          const blsSignatures = await BlsSignatures();
          const { PrivateKey: BlsPrivateKey, BasicSchemeMPL } = blsSignatures;

          const randomBytes = new Uint8Array(crypto.randomBytes(256));
          const operatorPrivateKey = BasicSchemeMPL.keyGen(randomBytes);

          const initialOperatorPrivateKey = Buffer.from(operatorPrivateKey.serialize()).toString('hex');

          function validateBLSPrivateKey(value) {
            if (value.length === 0) {
              return 'should not be empty';
            }

            const operatorPrivateKeyBuffer = Buffer.from(value, 'hex');

            let key;
            try {
              key = BlsPrivateKey.fromBytes(operatorPrivateKeyBuffer, true);
            } catch (e) {
              return 'invalid key';
            } finally {
              if (key) {
                key.delete();
              }
            }

            return true;
          }

          function validateRewardShare(value) {
            const reminder = value.split('.')[1];

            return Number(value) <= 100 && (!reminder || reminder.length <= 2);
          }

          function formatRewardShares(input, choice) {
            let str;

            const number = Number(input);
            if (Number.isNaN(number) || number.toFixed(2).length < input.length) {
              str = input;
            } else {
              str = number.toFixed(2);
            }

            const pos = Math.min(choice.cursor, str.length);

            const options = {
              input: str,
              initial: choice.initial,
              pos,
              showCursor: this.state.index === 1,
            };

            return placeholder(this, options);
          }

          const form = await task.prompt([
            // {
            //   type: 'form',
            //   header: 'Help user with collateral \n',
            //   message: 'Enter collateral information:',
            //   choices: [
            //     {
            //       name: 'txId',
            //       message: 'Transaction hash',
            //       validate: validateTxHash,
            //     },
            //     {
            //       name: 'outputIndex',
            //       message: 'Output index',
            //       validate: validateOutputIndex,
            //     },
            //   ],
            //   validate: ({ txId, outputIndex }) => validateTxHash(txId)
            //     && validateOutputIndex(outputIndex),
            // },
            // {
            //   type: 'form',
            //   header: 'Help user with these keys \n',
            //   message: 'Enter masternode keys and payout address:',
            //   choices: [
            //     {
            //       name: 'ownerPublicKey',
            //       message: 'Owner public key',
            //       validate: validateECDSAPublicKey,
            //     },
            //     {
            //       name: 'votingPublicKey',
            //       message: 'Voting public key',
            //       validate: validateECDSAPublicKey,
            //     },
            //     {
            //       name: 'payoutAddress',
            //       message: 'Payout address',
            //       validate: validateAddress,
            //     },
            //   ],
            //   validate: ({ ownerPublicKey, votingPublicKey, payoutAddress }) => (
            //     validateECDSAPublicKey(ownerPublicKey)
            //     && validateECDSAPublicKey(votingPublicKey)
            //     && validateAddress(payoutAddress)
            //   ),
            // },
            {
              type: 'form',
              header: 'Explain options with operator key and explain operator rewards\n',
              message: 'Please provide the following information:',
              choices: [
                {
                  name: 'privateKey',
                  message: 'BLS private key',
                  initial: initialOperatorPrivateKey,
                  validate: validateBLSPrivateKey,
                },
                {
                  name: 'rewardShare',
                  message: 'Reward share',
                  initial: '0.00',
                  validate: validateRewardShare,
                  format: formatRewardShares,
                  result: (value) => Number(value).toFixed(2),
                },
              ],
              validate: ({ privateKey, rewardShare }) => validateBLSPrivateKey(privateKey)
                && validateRewardShare(rewardShare),
            },
          ]);
        },
      },
      {
        title: 'Masternode operator key',
        enabled: (ctx) => ctx.isMasternodeRegistered,
        task: async (ctx, task) => {
          const blsSignatures = await BlsSignatures();
          const { PrivateKey: BlsPrivateKey } = blsSignatures;

          function validateBLSPrivateKey(value) {
            if (value.length === 0) {
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
                header: 'To import your masternode operator BLS private key, copy the\n'
                  + '"masternodeblsprivkey" field from your masternode\'s dash.conf file.\n',
                message: 'Enter BLS private key',
                validate: validateBLSPrivateKey,
              },
            ]);
          } else {
            const result = validateBLSPrivateKey(ctx.operatorBlsPrivateKey);

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
        title: 'Platform node key',
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
                header: 'Platform node key. Must be base64\n',
                message: 'Enter ED25519 key',
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

          // ctx.config.set('platform.drive.tenderdash.nodeKey', ctx.platformP2PKey);
        },
        options: {
          persistentOutput: true,
        },
      },
      {
        title: 'IP address and port',
        enabled: (ctx) => ctx.nodeType === NODE_TYPE_HPMN || ctx.nodeType === NODE_TYPE_MASTERNODE,
        task: async (ctx, task) => {
          if (ctx.externalIp === undefined) {

            const initialIp = !ctx.isMasternodeRegistered ? publicIp.v4() : undefined;

            function validateIp(ip) {
              return Boolean(ip.match(/^((25[0-5]|(2[0-4]|1\d|[1-9]|)\d)\.?\b){4}$/));
            }

            function validatePort(port) {
              const portNumber = Math.floor(Number(port));

              return portNumber >= 1
              && portNumber <= 65535
              && portNumber.toString() === port;
            }

            const form = await task.prompt([
              {
                type: 'form',
                header: 'The node external IP address must be static and will be used by the'
                  + ' network ..\n',
                message: 'Enter IP address and port:',
                choices: [
                  {
                    name: 'ip',
                    message: 'IPv4',
                    initial: initialIp,
                    validate: validateIp,
                  },
                  {
                    name: 'port',
                    message: 'Port',
                    initial: base.core.p2p.port.toString(),
                    validate: validatePort,
                  },
                ],
                validate: ({ ip, port }) => validateIp(ip) && validatePort(port),
              },
            ]);

            ctx.config.set('externalIp', form.ip);
            ctx.config.set('core.p2p.port', form.port);

            // eslint-disable-next-line no-param-reassign
            task.output = `${form.ip}:${form.port}`;
          }
        },
        options: {
          persistentOutput: true,
        },
      },
      {
        title: 'Set masternode operator BLS private key',
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
