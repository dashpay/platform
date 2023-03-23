const { Listr } = require('listr2');

const chalk = require('chalk');

const BlsSignatures = require('@dashevo/bls');

const {
  NODE_TYPE_MASTERNODE,
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
const createTenderdashNodeId = require('../../../../tenderdash/createTenderdashNodeId');

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
      OTHER: 'other',
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
                { name: REGISTRARS.OTHER, message: 'Other' },
              ],
              initial: REGISTRARS.CORE,
            },
          ]);

          let initialOperatorPrivateKey;
          if (registrar === REGISTRARS.CORE || registrar === REGISTRARS.OTHER) {
            ({ privateKey: initialOperatorPrivateKey } = await generateBlsKeys());
          }

          // TODO: We need to add description on how to find key generation form in the
          //  specified wallet

          // TODO: Deal with hints in forms

          // TODO: When registering a new masternode, if registration transaction was not successful, then going back through the previous steps should already contain the previously filled in information

          const validateAddressWithNetwork = (value) => validateAddress(value, ctx.preset);

          const collateralAmount = ctx.nodeType === NODE_TYPE_MASTERNODE ? MASTERNODE_COLLATERAL_AMOUNT : HPMN_COLLATERAL_AMOUNT;
          const collateralDenomination = ctx.preset === PRESET_MAINNET ? 'DASH' : 'tDASH';

          const prompts = [
            {
              type: 'form',
              name: 'collateral',
              header: `  Dashmate needs to collect your collateral funding transaction hash and index.
  The funding value must be exactly ${collateralAmount} ${collateralDenomination}.\n`,
              message: 'Enter collateral funding transaction information:',
              choices: [
                {
                  name: 'txId',
                  message: 'Transaction hash',
                  validate: validateTxHex,
                },
                {
                  name: 'outputIndex',
                  message: 'Output index',
                  validate: validatePositiveInteger,
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
  addresses, encoded in base58 format.\n`,
              message: 'Enter DIP3 masternode addresses:',
              choices: [
                {
                  name: 'ownerAddress',
                  message: 'Owner address',
                  hint: 'Base58 encoded',
                  validate: validateAddressWithNetwork,
                },
                {
                  name: 'votingAddress',
                  message: 'Voting address',
                  hint: 'Base58 encoded',
                  validate: validateAddressWithNetwork,
                },
                {
                  name: 'payoutAddress',
                  message: 'Payout address',
                  hint: 'Base58 encoded',
                  validate: validateAddressWithNetwork,
                },
              ],
              validate: ({ ownerAddress, votingAddress, payoutAddress }) => {
                if (ownerAddress === payoutAddress || votingAddress === payoutAddress) {
                  return 'The payout address may not be the same as the owner or voting address';
                }

                return validateAddressWithNetwork(ownerAddress)
                  && validateAddressWithNetwork(votingAddress)
                  && validateAddressWithNetwork(payoutAddress);
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
                  hint: 'HEX encoded',
                  initial: initialOperatorPrivateKey,
                  validate: validateBLSPrivateKey,
                },
                {
                  name: 'rewardShare',
                  message: 'Reward share',
                  hint: '%',
                  initial: '0.00',
                  validate: validatePercentage,
                  format: formatPercentage,
                  result: (value) => Number(value).toFixed(2),
                },
              ],
              validate: ({ privateKey, rewardShare }) => validateBLSPrivateKey(privateKey)
                && validatePercentage(rewardShare),
            },
          ];

          if (ctx.isHP) {
            prompts.push(createPlatformNodeKeyInput());
          }

          prompts.push(await createIpAndPortsForm(ctx.preset, {
            isHPMN: ctx.isHP,
          }));

          let form;
          let confirmation;
          do {
            form = await task.prompt(prompts);

            const operatorPrivateKeyBuffer = Buffer.from(form.operator.privateKey, 'hex');
            const operatorPrivateKey = blsSignatures.PrivateKey.fromBytes(
              operatorPrivateKeyBuffer,
              true,
            );
            const operatorPublicKey = operatorPrivateKey.getG1();
            const operatorPublicKeyHex = Buffer.from(operatorPublicKey.serialize()).toString('hex');

            const platformP2PPort = form.ipAndPorts.platformP2PPort
              || systemConfigs[ctx.preset].platform.drive.tenderdash.p2p.port;

            const platformHTTPPort = form.ipAndPorts.platformHTTPPort
              || systemConfigs[ctx.preset].platform.dapi.envoy.http.port;

            let command;
            if (ctx.isHP) {
              command = `dash-cli register_hpmn
                    ${form.collateral.txId}
                    ${form.collateral.outputIndex}
                    ${form.ipAndPorts.ip}:${form.ipAndPorts.coreP2PPort}
                    ${form.keys.ownerAddress}
                    ${operatorPublicKeyHex}
                    ${form.keys.votingAddress}
                    ${form.operator.rewardShare}
                    ${form.keys.payoutAddress}
                    ${createTenderdashNodeId(form.platformNodeKey)}
                    ${platformP2PPort}
                    ${platformHTTPPort}`;
            } else {
              command = `dash-cli register
                    ${form.collateral.txId}
                    ${form.collateral.outputIndex}
                    ${form.ipAndPorts.ip}:${form.ipAndPorts.coreP2PPort}
                    ${form.keys.ownerAddress}
                    ${operatorPublicKeyHex}
                    ${form.keys.votingAddress}
                    ${form.operator.rewardShare}
                    ${form.keys.payoutAddress}`;
            }

            confirmation = await task.prompt({
              type: 'toggle',
              name: 'confirm',
              header: chalk`  Now run the following command to create the registration transaction:
              {bold.cyanBright ${command}}

  Select "No" to modify the transaction by amending your previous input.\n`,
              message: 'Was the masternode registration transaction successful?',
              enabled: 'Yes',
              disabled: 'No',
            });
          } while (!confirmation);

          ctx.config.set('core.masternode.operator.privateKey', form.operator.privateKey);

          ctx.config.set('externalIp', form.ipAndPorts.ip);
          ctx.config.set('core.p2p.port', form.ipAndPorts.coreP2PPort);

          if (ctx.isHP) {
            ctx.config.set('platform.drive.tenderdash.node.id', createTenderdashNodeId(form.platformNodeKey));
            ctx.config.set('platform.drive.tenderdash.node.key', form.platformNodeKey);

            ctx.config.set('platform.dapi.envoy.http.port', form.ipAndPorts.platformHTTPPort);
            ctx.config.set('platform.drive.tenderdash.p2p.port', form.ipAndPorts.platformP2PPort);
          }

          // TODO: Output configuration
        },
      },
    ]);
  }

  return registerMasternodeGuideTask;
}

module.exports = registerMasternodeGuideTaskFactory;
