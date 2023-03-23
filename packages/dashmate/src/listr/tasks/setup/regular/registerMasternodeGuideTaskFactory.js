const { Listr } = require('listr2');

const chalk = require('chalk');

const BlsSignatures = require('@dashevo/bls');

const validateAddressHex = require('../../../prompts/validators/validateAddressHex');
const validateTxHex = require('../../../prompts/validators/validateTxHex');
const validatePositiveInteger = require('../../../prompts/validators/validatePositiveInteger');
const validatePercentage = require('../../../prompts/validators/validatePercentage');
const formatPercentage = require('../../../prompts/formatters/formatPercentage');
const generateBlsKeys = require('../../../../core/generateBlsKeys');
const validateBLSPrivateKeyFactory = require('../../../prompts/validators/validateBLSPrivateKeyFactory');
const createPlatformNodeKeyInput = require('../../../prompts/createPlatformNodeKeyInput');
const createIpAndPortsForm = require('../../../prompts/createIpAndPortsForm');

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

          const validateAddressHexWithNetwork = (value) => validateAddressHex(value, ctx.preset);

          const prompts = [
            {
              type: 'form',
              name: 'collateral',
              header: 'Dashmate needs to collect details about your collateral funding'
                + ' transaction. The funding value must be exactly 1000 DASH (masternode)'
                + ' or 4000 DASH (high-performance masternode).\n',
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
              header: 'Dashmate needs to collect details about the owner, voting and payout'
                + ' addresses to use in the masternode registration transaction. These are'
                + ' regular Dash addresses, encoded in HEX format.\n',
              message: 'Enter DIP3 masternode addresses:',
              choices: [
                {
                  name: 'ownerAddress',
                  message: chalk` Owner address {gray HEX encoded}`,
                  network: ctx.preset,
                  validate: validateAddressHexWithNetwork,
                },
                {
                  name: 'votingAddress',
                  message: chalk` Voting address {gray HEX encoded}`,
                  network: ctx.preset,
                  validate: validateAddressHexWithNetwork,
                },
                {
                  name: 'payoutAddress',
                  message: chalk` Payout address {gray HEX encoded}`,
                  network: ctx.preset,
                  validate: validateAddressHexWithNetwork,
                },
              ],
              validate: ({ ownerAddress, votingAddress, payoutAddress }) => {
                if (ownerAddress === payoutAddress || votingAddress === payoutAddress) {
                  return 'The payout address may not be the same as the owner or voting address';
                }

                return validateAddressHexWithNetwork(ownerAddress)
                  && validateAddressHexWithNetwork(votingAddress)
                  && validateAddressHexWithNetwork(payoutAddress);
              },
            },
            {
              type: 'form',
              name: 'operator',
              header: 'Dashmate needs to collect details on the operator key and operator'
                + ' reward share to use in the registration transaction. The operator key is'
                + ' a BLS private key, encoded in HEX format. Dashmate will record the private'
                + ' key in the masternode configuration, and derive the public key for use in'
                + ' the masternode registration transaction. You may optionally also specify a'
                + ' percentage share of the masternode reward to pay to the operator.\n',
              message: 'Enter masternode operator private key and reward share:',
              choices: [
                {
                  name: 'privateKey',
                  message: chalk`BLS private key {gray HEX encoded}`,
                  initial: initialOperatorPrivateKey,
                  validate: validateBLSPrivateKey,
                },
                {
                  name: 'rewardShare',
                  message: chalk`Reward share %`,
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

          prompts.push(await createIpAndPortsForm({
            isHPMN: ctx.isHP,
          }));

          let form;
          let confirmation;
          do {
            form = await task.prompt(prompts);

            confirmation = await task.prompt({
              type: 'toggle',
              name: 'confirm',
              header: chalk` You should run the command:
              {bold.green dash-cli ${ctx.isHP ? 'register_hpmn' : 'register'}
                    argument1
                    argument2
              }

              Go with nope to come back to edit command\n`,
              message: 'Have you registered a masternode successfully?',
              enabled: 'Yep',
              disabled: 'Nope',
            });
          } while (!confirmation);

          // TODO: Store form information to the config
          console.dir(form);

          // ctx.config.set('externalIp', form.ip);
          // ctx.config.set('core.p2p.port', form.port);
          // TODO: Derive node id from key
          // config.set('platform.drive.tenderdash.nodeId', nodeId);

          // ctx.config.set('platform.drive.tenderdash.nodeKey', ctx.platformP2PKey);

          // TODO: Output configuration
        },
      },
    ]);
  }

  return registerMasternodeGuideTask;
}

module.exports = registerMasternodeGuideTaskFactory;
