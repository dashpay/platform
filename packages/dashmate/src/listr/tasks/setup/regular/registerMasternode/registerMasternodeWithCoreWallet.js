const wrapAnsi = require('wrap-ansi');
const chalk = require('chalk');
const BlsSignatures = require('@dashevo/bls');
const generateBlsKeys = require('../../../../../core/generateBlsKeys');
const validateAddress = require('../../../../prompts/validators/validateAddress');
const {
  HPMN_COLLATERAL_AMOUNT,
  MASTERNODE_COLLATERAL_AMOUNT,
  PRESET_MAINNET,
} = require('../../../../../constants');

const validateTxHex = require('../../../../prompts/validators/validateTxHex');
const validatePositiveInteger = require('../../../../prompts/validators/validatePositiveInteger');
const validatePercentage = require('../../../../prompts/validators/validatePercentage');
const formatPercentage = require('../../../../prompts/formatters/formatPercentage');
const createPlatformNodeKeyInput = require('../../../../prompts/createPlatformNodeKeyInput');
const getBLSPublicKeyFromPrivateKeyHex = require('../../../../../core/getBLSPublicKeyFromPrivateKeyHex');
const deriveTenderdashNodeId = require('../../../../../tenderdash/deriveTenderdashNodeId');
const validateBLSPrivateKeyFactory = require('../../../../prompts/validators/validateBLSPrivateKeyFactory');
const providers = require("../../../../../status/providers");
const PortStatusEnum = require("../../../../../status/enums/portState");

/**
 * @param {createIpAndPortsForm} createIpAndPortsForm
 * @return {registerMasternodeWithCoreWallet}
 */
function registerMasternodeWithCoreWalletFactory(createIpAndPortsForm, resolvePublicIpV4) {
  /**
   * Print prompts to collect masternode registration data with Core
   *
   * @typedef {function} registerMasternodeWithCoreWallet
   * @param {Context} ctx
   * @param {TaskWrapper} task
   * @param {DefaultConfigs} defaultConfigs
   * @returns {Promise<{
   *   keys: {},
   *   ipAndPorts: {
   *      platformP2PPort: null,
   *      coreP2PPort: null,
   *      platformHTTPPort: null
   *   },
   *   collateral: {},
   *   operator: {
   *      rewardShare: null,
   *      privateKey: null
   *   },
   *   platformNodeKey: null
   * }>}
   */
  async function registerMasternodeWithCoreWallet(ctx, task, defaultConfigs) {
    const blsSignatures = await BlsSignatures();
    const validateBLSPrivateKey = validateBLSPrivateKeyFactory(blsSignatures);

    const validateAddressWithNetwork = (value) => validateAddress(value, ctx.preset);

    const collateralAmount = ctx.isHP === true
      ? HPMN_COLLATERAL_AMOUNT
      : MASTERNODE_COLLATERAL_AMOUNT;

    const collateralDenomination = ctx.preset === PRESET_MAINNET ? 'DASH' : 'tDASH';

    let state = {
      collateral: {},
      keys: {},
      operator: {
        privateKey: null,
        rewardShare: null,
      },
      ipAndPorts: {
        coreP2PPort: null,
        platformHTTPPort: null,
        platformP2PPort: null,
      },
      platformNodeKey: null,
    };

    let instructionsUrl = 'https://docs.dash.org/mn-setup-core-collateral';
    if (ctx.isHP) {
      instructionsUrl = 'https://docs.dash.org/evonode-setup-core-collateral';
    }

    let confirmation;
    do {
      const { privateKey: initialOperatorPrivateKey } = await generateBlsKeys();

      const prompts = [
        {
          type: 'form',
          name: 'collateral',
          header: `  Dashmate needs to collect your collateral funding transaction hash and index.
  The funding value must be exactly ${collateralAmount} ${collateralDenomination}.

  Please follow the instructions on how to create a collateral funding transaction in Dash Core Wallet:
  ${instructionsUrl}\n`,
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
          validate: ({
            txId,
            outputIndex,
          }) => validateTxHex(txId)
            && validatePositiveInteger(outputIndex),
        },
        {
          type: 'form',
          name: 'keys',
          header: `  Dashmate needs to collect details about the owner, voting and payout addresses
  to use in the masternode registration transaction. These are regular Dash
  addresses, encoded in base58 format.\n`,
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
          validate: ({
            ownerAddress,
            votingAddress,
            payoutAddress,
          }) => {
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
  encoded in hexadecimal format. Dashmate will record the private key in the masternode
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
              result: (value) => Number(value)
                .toFixed(2),
            },
          ],
          validate: ({
            privateKey,
            rewardShare,
          }) => (
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

      const portStatus  = await providers.mnowatch.checkPortStatus(state.ipAndPorts.coreP2PPort)

      if (portStatus !== PortStatusEnum.OPEN) {
        const externalIp = await resolvePublicIpV4() ?? 'unresolved'

        const confirmation = await task.prompt({
          type: 'toggle',
          name: 'confirm',
          header: `You have chosen Core P2P port ${state.ipAndPorts.coreP2PPort}, ` +
`however it looks not reachable on your host ` +
`${chalk.red(`(TCP ${externalIp}:${state.ipAndPorts.coreP2PPort} ${portStatus})`)}`,
          message: 'Are you sure that you want to continue?',
          enabled: 'Yes',
          disabled: 'No',
        });

        if (!confirmation) {
          throw new Error('Operation is cancelled')
        }
      }

      const operatorPublicKeyHex = await getBLSPublicKeyFromPrivateKeyHex(
        state.operator.privateKey,
      );

      const platformP2PPort = state.ipAndPorts.platformP2PPort
        || defaultConfigs.get(ctx.preset)
          .get('platform.drive.tenderdash.p2p.port');

      const platformHTTPPort = state.ipAndPorts.platformHTTPPort
        || defaultConfigs.get(ctx.preset)
          .get('platform.dapi.envoy.http.port');

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
        command = wrapAnsi(command, process.stdout.columns - 3, {
          hard: true,
          trim: false,
        });
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

    return state;
  }

  return registerMasternodeWithCoreWallet;
}

module.exports = registerMasternodeWithCoreWalletFactory;
