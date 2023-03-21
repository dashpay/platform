const { Listr } = require('listr2');

const styles = require('enquirer/lib/styles');
const chalk = require('chalk');

const { Observable } = require('rxjs');

const {
  NODE_TYPE_MASTERNODE,
  NODE_TYPE_HPMN,
} = require('../../../../constants');

const crypto = require('crypto');

const publicIp = require('public-ip');

const BlsSignatures = require('@dashevo/bls');

const { PrivateKey, PublicKey, Address } = require('@dashevo/dashcore-lib');

const placeholder = require('enquirer/lib/placeholder');
const createTenderdashNodeId = require('../../../../tenderdash/createTenderdashNodeId');
const generateTenderdashNodeKey = require('../../../../tenderdash/generateTenderdashNodeKey');
const validateTenderdashNodeKey = require('../../../prompts/validators/validateTenderdashNodeKey');
const validateAddressHex = require('../../../prompts/validators/validateAddressHex');

/**
 *
 * @param {startCore} startCore
 * @param {createNewAddress} createNewAddress
 * @param {generateToAddress} generateToAddress
 * @param {generateBlocks} generateBlocks
 * @param {waitForCoreSync} waitForCoreSync
 * @param {importPrivateKey} importPrivateKey
 * @param {getAddressBalance} getAddressBalance
 * @param {sendToAddress} sendToAddress
 * @param {waitForConfirmations} waitForConfirmations
 * @param {registerMasternode} registerMasternode
 * @param {waitForBalanceToConfirm} waitForBalanceToConfirm
 * @param {createIpAndPortsForm} createIpAndPortsForm
 * @return {registerMasternodeGuideTask}
 */
function registerMasternodeGuideTaskFactory(
  startCore,
  createNewAddress,
  generateToAddress,
  generateBlocks,
  waitForCoreSync,
  importPrivateKey,
  getAddressBalance,
  sendToAddress,
  waitForConfirmations,
  registerMasternode,
  waitForBalanceToConfirm,
  createIpAndPortsForm,
) {
  /**
   * @typedef {registerMasternodeGuideTask}
   * @return {Listr}
   */
  async function registerMasternodeGuideTask() {
    const blsSignatures = await BlsSignatures();
    const { PrivateKey: BlsPrivateKey, BasicSchemeMPL } = blsSignatures;

    const REGISTRARS = {
      CORE: 'dashDesktop',
      ANDROID: 'dashAndroid',
      IOS: 'dashIOS',
      OTHER: 'other',
    };

    const registrarList = [
      { name: REGISTRARS.CORE, message: 'Desktop Dash Wallet' },
      { name: REGISTRARS.ANDROID, message: 'Android Dash Wallet' },
      { name: REGISTRARS.IOS, message: 'iOS Dash Wallet' },
      { name: REGISTRARS.OTHER, message: 'Other' },
    ];

    const registrarMap = registrarList.reduce((obj, { name, message }) => {
      // eslint-disable-next-line no-param-reassign
      obj[name] = message;

      return obj;
    }, {});


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




    return new Listr([
      {
        task: async (ctx, task) => {
          ctx.registrar = await task.prompt([
            {
              type: 'select',
              header: 'For security reasons, Dash masternodes should never store masternode owner'
                + ' or collateral private keys. Dashmate therefore cannot register a masternode for'
                + ' you directly. Instead, we will generate RPC commands that you can use in '
                + ' Desktop Dash Wallet (Dash Core)'
                + ' or other external tools where the keys are handled securely. During this'
                + ' process, dashmate can optionally generate configuration elements as necessary,'
                + ' such the BLS operator key and the node id, since this is the'
                + ' only information necessary for dashmate to configure the masternode.\n',
              message: 'Which wallet will you use to store keys for your masternode?',
              choices: registrarList,
              initial: REGISTRARS.CORE,
            },
          ]);
        },
      },
      {
        title: 'Register masternode',
        enabled: (ctx) => !ctx.isMasternodeRegistered
          && (ctx.nodeType === NODE_TYPE_HPMN || ctx.nodeType === NODE_TYPE_MASTERNODE),
        task: async (ctx, task) => {
          // eslint-disable-next-line no-param-reassign
          task.title = `Register masternode with ${registrarMap[ctx.registrar]}`;

          let initialOperatorPrivateKey;
          if (ctx.registrar === REGISTRARS.CORE || ctx.registrar === REGISTRARS.OTHER) {
            const randomBytes = new Uint8Array(crypto.randomBytes(256));
            const operatorPrivateKey = BasicSchemeMPL.keyGen(randomBytes);

            initialOperatorPrivateKey = Buffer.from(operatorPrivateKey.serialize()).toString('hex');
          }

          // TODO: We need to add description on how to find key generation form in the
          //  specified wallet


          // TODO: Implement additional validations
          /*
           ipAddress is set and port is not set to the default mainnet port
           ipAddress is set and not routable or not an IPv4 mapped address
           ipAddress is set and already used in the registered masternodes set
           */

          const validateAddressHexWithNetwork = (value) => validateAddressHex(value, ctx.preset);

          const prompts = [
            {
              type: 'form',
              name: 'collateral',
              header: 'Help user with collateral \n',
              message: 'Enter collateral information:',
              choices: [
                {
                  name: 'txId',
                  message: 'Transaction hash',
                  validate: validateTxHash,
                },
                {
                  name: 'outputIndex',
                  message: 'Output index',
                  validate: validateOutputIndex,
                },
              ],
              validate: ({ txId, outputIndex }) => validateTxHash(txId)
                && validateOutputIndex(outputIndex),
            },
            {
              type: 'form',
              name: 'keys',
              header: 'Help user with these keys \n',
              message: 'Enter masternode addresses:',
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
                  return 'you should use different payout address';
                }

                return validateAddressHexWithNetwork(ownerAddress)
                  && validateAddressHexWithNetwork(votingAddress)
                  && validateAddressHexWithNetwork(payoutAddress);
              },
            },
            {
              type: 'form',
              name: 'operator',
              header: 'Explain options with operator key and explain operator rewards\n',
              message: 'Please provide the following information:',
              choices: [
                {
                  name: 'privateKey',
                  message: chalk`BLS private key HEX encoded`,
                  initial: initialOperatorPrivateKey,
                  validate: validateBLSPrivateKey,
                },
                {
                  name: 'rewardShare',
                  message: chalk`Reward shares %`,
                  initial: '0.00',
                  validate: validateRewardShare,
                  format: formatRewardShares,
                  result: (value) => Number(value).toFixed(2),
                },
              ],
              validate: ({ privateKey, rewardShare }) => validateBLSPrivateKey(privateKey)
                && validateRewardShare(rewardShare),
            },
          ];

          if (ctx.nodeType === NODE_TYPE_HPMN) {
            prompts.push({
              type: 'input',
              name: 'platformNodeKey',
              header: 'Platform node ED25519 key. What\'s that. Must be base64 tendermint' +
                ' format\n',
              message: 'Enter Node key',
              hint: 'Base64 encoded',
              initial: generateTenderdashNodeKey(),
              validate: validateTenderdashNodeKey,
            });
          }

          prompts.push(await createIpAndPortsForm({
            isHPMN: ctx.nodeType === NODE_TYPE_HPMN,
          }));


          let form;
          let confirmation;
          do {
            form = await task.prompt(prompts);

            confirmation = await task.prompt([
              {
                type: 'toggle',
                name: 'confirm',
                header: chalk` You should run the command:
                {bold.green dash-cli ${ctx.nodeType === NODE_TYPE_HPMN ? 'register_hpmn' : 'register'}
                      argument1
                      argument2
                }
                
                Go with nope to come back to edit command\n`,
                message: 'Have you registered a masternode successfully?',
                enabled: 'Yep',
                disabled: 'Nope',
              },
            ]);
          } while (!confirmation);

          // TODO: Store form information to the config
          console.dir(form);
        },
      },
    ]);
  }

  return registerMasternodeGuideTask;
}

module.exports = registerMasternodeGuideTaskFactory;
