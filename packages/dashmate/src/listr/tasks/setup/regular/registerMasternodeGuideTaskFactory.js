const { Listr } = require('listr2');
const wrapAnsi = require('wrap-ansi');

const chalk = require('chalk');

const BlsSignatures = require('@dashevo/bls');

const {
  MASTERNODE_COLLATERAL_AMOUNT,
  HPMN_COLLATERAL_AMOUNT,
  PRESET_MAINNET,
} = require('../../../../constants');

const systemConfigs = require('../../../../../configs/system');

const validateAddress = require('../../../prompts/validators/validateAddress');
const validateTxHex = require('../../../prompts/validators/validateTxHex');
const validatePositiveInteger = require('../../../prompts/validators/validatePositiveInteger');
const validatePercentage = require('../../../prompts/validators/validatePercentage');
const formatPercentage = require('../../../prompts/formatters/formatPercentage');
const generateBlsKeys = require('../../../../core/generateBlsKeys');
const validateBLSPrivateKeyFactory = require('../../../prompts/validators/validateBLSPrivateKeyFactory');
const createPlatformNodeKeyInput = require('../../../prompts/createPlatformNodeKeyInput');
const createIpAndPortsForm = require('../../../prompts/createIpAndPortsForm');
const deriveTenderdashNodeId = require('../../../../tenderdash/deriveTenderdashNodeId');
const getConfigurationOutputFromContext = require('./getConfigurationOutputFromContext');
const getBLSPublicKeyFromPrivateKeyHex = require('../../../../core/getBLSPublicKeyFromPrivateKeyHex');

/**
 * @return {registerMasternodeGuideTask}
 */
function registerMasternodeGuideTaskFactory() {
  /**
   * @typedef {registerMasternodeGuideTask}
   * @return {Listr}
   */
  async function registerMasternodeGuideTask() {
    const blsSignatures = await BlsSignatures();

    const validateBLSPrivateKey = validateBLSPrivateKeyFactory(blsSignatures);

    const REGISTRARS = {
      CORE: 'dashCore',
      ANDROID: 'dashAndroid',
      IOS: 'dashIOS',
      DMT: 'dmt',
    };

    return new Listr([
      {
        title: 'Register masternode',
        task: async (ctx, task) => {
          const registrar = await task.prompt([
            {
              type: 'select',
              header: `  For security reasons, Dash masternodes should never store masternode owner or
  collateral private keys. Dashmate therefore cannot register a masternode for you
  directly. Instead, we will generate RPC commands that you can use in Dash Core
  or other external tools where keys are handled securely. During this process,
  dashmate can optionally generate configuration elements as necessary, such as
  the BLS operator key and the node id.\n`,
              message: 'Which wallet will you use to store keys for your masternode?',
              choices: [
                { name: REGISTRARS.CORE, message: 'Dash Core Wallet' },
                { name: REGISTRARS.ANDROID, message: 'Android Dash Wallet' },
                { name: REGISTRARS.IOS, message: 'iOS Dash Wallet' },
                { name: REGISTRARS.DMT, message: 'DMT' },
              ],
              initial: REGISTRARS.CORE,
            },
          ]);

          let initialOperatorPrivateKey;
          if (registrar === REGISTRARS.CORE || registrar === REGISTRARS.DMT) {
            ({ privateKey: initialOperatorPrivateKey } = await generateBlsKeys());
          }

          // TODO: We need to add description on how to find key generation form in the
          //  specified wallet

          const instructions = {
            [REGISTRARS.DMT]: {
              collateral: '\n  Click Locate collateral and copy the transaction hash and output index.',
              keys: '\n  Copy these values from the relevant fields in the form',
            },
            [REGISTRARS.CORE]: {
              collateral: '',
              keys: '',
            },
            [REGISTRARS.ANDROID]: {
              collateral: '',
              keys: '',
            },
            [REGISTRARS.IOS]: {
              collateral: '',
              keys: '',
            },
          };

          const validateAddressWithNetwork = (value) => validateAddress(value, ctx.preset);

          const collateralAmount = ctx.isHP === true
            ? HPMN_COLLATERAL_AMOUNT
            : MASTERNODE_COLLATERAL_AMOUNT;

          const collateralDenomination = ctx.preset === PRESET_MAINNET ? 'DASH' : 'tDASH';

          let state = {
            collateral: {},
            keys: {},
            operator: {},
            ipAndPorts: {},
          };

          let confirmation;
          do {
            const prompts = [
              {
                type: 'form',
                name: 'collateral',
                header: `  Dashmate needs to collect your collateral funding transaction hash and index.
  The funding value must be exactly ${collateralAmount} ${collateralDenomination}.\n
  ${instructions[registrar].collateral}\n
  `,
                message: 'Enter collateral funding transaction information:',
                choices: [
                  {
                    name: 'txId',
                    message: 'Transaction hash',
                    validate: validateTxHex,
                    initial: state.collateral.txId,
                  },
                  {
                    name: 'outputIndex',
                    message: 'Output index',
                    validate: validatePositiveInteger,
                    initial: state.collateral.outputIndex,
                  },
                ],
                validate: ({ txId, outputIndex }) => validateTxHex(txId)
                  && validatePositiveInteger(outputIndex),
              },
              {
                type: 'form',
                name: 'keys',
                header: `  Dashmate needs to collect details about the owner, voting and payout addresses
  to use in the masternode registration transaction. These are regular Dash
  addresses, encoded in base58 format.\n${instructions[registrar].keys}\n}`,
                message: 'Enter masternode addresses:',
                choices: [
                  {
                    name: 'ownerAddress',
                    message: 'Owner address',
                    validate: validateAddressWithNetwork,
                    initial: state.keys.ownerAddress,
                  },
                  {
                    name: 'votingAddress',
                    message: 'Voting address',
                    validate: validateAddressWithNetwork,
                    initial: state.keys.votingAddress,
                  },
                  {
                    name: 'payoutAddress',
                    message: 'Payout address',
                    validate: validateAddressWithNetwork,
                    initial: state.keys.payoutAddress,
                  },
                ],
                validate: ({ ownerAddress, votingAddress, payoutAddress }) => {
                  if (!validateAddressWithNetwork(ownerAddress)
                    || !validateAddressWithNetwork(votingAddress)
                    || !validateAddressWithNetwork(payoutAddress)) {
                    return false;
                  }

                  if (ownerAddress === payoutAddress || votingAddress === payoutAddress) {
                    return 'The payout address may not be the same as the owner or voting address';
                  }

                  return true;
                },
              },
              {
                type: 'form',
                name: 'operator',
                header: `  Dashmate needs to collect details on the operator key and operator reward share
  to use in the registration transaction. The operator key is a BLS private key,
  encoded in HEX format. Dashmate will record the private key in the masternode
  configuration, and derive the public key for use in the masternode registration
  transaction. You may optionally also specify a percentage share of the
  masternode reward to pay to the operator.\n`,
                message: 'Enter masternode operator private key and reward share:',
                choices: [
                  {
                    name: 'privateKey',
                    message: 'BLS private key',
                    initial: state.operator.privateKey || initialOperatorPrivateKey,
                    validate: validateBLSPrivateKey,
                  },
                  {
                    name: 'rewardShare',
                    message: 'Reward share',
                    hint: '%',
                    initial: state.operator.rewardShare || '0.00',
                    validate: validatePercentage,
                    format: formatPercentage,
                    result: (value) => Number(value).toFixed(2),
                  },
                ],
                validate: ({ privateKey, rewardShare }) => (
                  validateBLSPrivateKey(privateKey) === true && validatePercentage(rewardShare)),
              },
            ];

            if (ctx.isHP) {
              prompts.push(createPlatformNodeKeyInput({
                initial: state.platformNodeKey,
              }));
            }

            prompts.push(await createIpAndPortsForm(ctx.preset, {
              isHPMN: ctx.isHP,
              initialIp: state.ipAndPorts.ip,
              initialCoreP2PPort: state.ipAndPorts.coreP2PPort,
              initialPlatformP2PPort: state.ipAndPorts.platformP2PPort,
              initialPlatformHTTPPort: state.ipAndPorts.platformHTTPPort,
            }));

            state = await task.prompt(prompts);

            const operatorPublicKeyHex = await getBLSPublicKeyFromPrivateKeyHex(
              state.operator.privateKey,
            );

            const platformP2PPort = state.ipAndPorts.platformP2PPort
              || systemConfigs[ctx.preset].platform.drive.tenderdash.p2p.port;

            const platformHTTPPort = state.ipAndPorts.platformHTTPPort
              || systemConfigs[ctx.preset].platform.dapi.envoy.http.port;

            let command;
            if (ctx.isHP) {
              command = `dash-cli protx register_hpmn \\
  ${state.collateral.txId} \\
  ${state.collateral.outputIndex} \\
  ${state.ipAndPorts.ip}:${state.ipAndPorts.coreP2PPort} \\
  ${state.keys.ownerAddress} \\
  ${operatorPublicKeyHex} \\
  ${state.keys.votingAddress} \\
  ${state.operator.rewardShare} \\
  ${state.keys.payoutAddress} \\
  ${deriveTenderdashNodeId(state.platformNodeKey)} \\
  ${platformP2PPort} \\
  ${platformHTTPPort}`;
            } else {
              command = `dash-cli protx register \\
  ${state.collateral.txId} \\
  ${state.collateral.outputIndex} \\
  ${state.ipAndPorts.ip}:${state.ipAndPorts.coreP2PPort} \\
  ${state.keys.ownerAddress} \\
  ${operatorPublicKeyHex} \\
  ${state.keys.votingAddress} \\
  ${state.operator.rewardShare} \\
  ${state.keys.payoutAddress}`;
            }

            // Wrap the command to fit the terminal width (listr uses new lines to wrap the text)
            if (!ctx.isVerbose) {
              command = command.replace(/\\/g, '');
              command = wrapAnsi(command, process.stdout.columns - 3, { hard: true, trim: false });
              command = command.replace(/\n/g, '\\\n');
            }

            // TODO: We need to give more info on how to run this command

            confirmation = await task.prompt({
              type: 'toggle',
              name: 'confirm',
              header: chalk`  Now run the following command to create the registration transaction:

  {bold.cyanBright ${command}}

  Select "No" to modify the command by amending your previous input.\n`,
              message: 'Was the masternode registration transaction successful?',
              enabled: 'Yes',
              disabled: 'No',
            });
          } while (!confirmation);

          ctx.config.set('core.masternode.operator.privateKey', state.operator.privateKey);

          ctx.config.set('externalIp', state.ipAndPorts.ip);
          ctx.config.set('core.p2p.port', state.ipAndPorts.coreP2PPort);

          if (ctx.isHP) {
            ctx.config.set('platform.drive.tenderdash.node.id', deriveTenderdashNodeId(state.platformNodeKey));
            ctx.config.set('platform.drive.tenderdash.node.key', state.platformNodeKey);

            ctx.config.set('platform.dapi.envoy.http.port', state.ipAndPorts.platformHTTPPort);
            ctx.config.set('platform.drive.tenderdash.p2p.port', state.ipAndPorts.platformP2PPort);
          }

          // eslint-disable-next-line no-param-reassign
          task.output = await getConfigurationOutputFromContext(ctx);
        },
        options: {
          persistentOutput: true,
        },
      },
    ]);
  }

  return registerMasternodeGuideTask;
}

module.exports = registerMasternodeGuideTaskFactory;
