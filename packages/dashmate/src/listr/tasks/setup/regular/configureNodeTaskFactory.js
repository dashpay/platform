const { Listr } = require('listr2');

const BlsSignatures = require('@dashevo/bls');

const {
  NODE_TYPE_MASTERNODE,
  PRESET_MAINNET,
  NODE_TYPE_FULLNODE,
} = require('../../../../constants');

const validateBLSPrivateKeyFactory = require('../../../prompts/validators/validateBLSPrivateKeyFactory');
const createPlatformNodeKeyInput = require('../../../prompts/createPlatformNodeKeyInput');
const deriveTenderdashNodeId = require('../../../../tenderdash/deriveTenderdashNodeId');
const getConfigurationOutputFromContext = require('./getConfigurationOutputFromContext');

/**
 *
 * @param {createIpAndPortsForm} createIpAndPortsForm
 * @return {configureNodeTask}
 */
function configureNodeTaskFactory(createIpAndPortsForm) {
  /**
   * @typedef {function} configureNodeTask
   * @returns {Listr}
   */
  async function configureNodeTask() {
    const blsSignatures = await BlsSignatures();

    const validateBLSPrivateKey = validateBLSPrivateKeyFactory(blsSignatures);

    return new Listr([
      {
        title: 'Configure node',
        task: async (ctx, task) => {
          // eslint-disable-next-line no-param-reassign
          task.title = `Configure ${ctx.nodeType}`;

          // Masternode Operator key
          if (ctx.nodeType === NODE_TYPE_MASTERNODE) {
            const masternodeOperatorPrivateKey = await task.prompt({
              type: 'input',
              header: `  Please enter your Masternode Operator BLS Private key.

  Your BLS private key can be found in the "dash.conf" file of your masternode,
  in the DMT configuration tool, or in the safe location in which you stored it
  when initially configuring your masternode.\n`,
              message: 'BLS private key',
              validate: validateBLSPrivateKey,
            });

            ctx.config.set('core.masternode.operator.privateKey', masternodeOperatorPrivateKey);
          }

          // Platform Node Key
          if (ctx.isHP) {
            let platformNodeKey = ctx.tenderdashNodeKey;
            if (!ctx.tenderdashNodeKey) {
              platformNodeKey = await task.prompt(createPlatformNodeKeyInput({
                initial: ctx.nodeType === NODE_TYPE_MASTERNODE ? '' : undefined,
              }));
            }

            ctx.config.set('platform.drive.tenderdash.node.id', deriveTenderdashNodeId(platformNodeKey));
            ctx.config.set('platform.drive.tenderdash.node.key', platformNodeKey);
          }

          // IP and ports
          if (
            ctx.nodeType === NODE_TYPE_MASTERNODE
            || (ctx.nodeType === NODE_TYPE_FULLNODE && ctx.isHP)
          ) {
            const showEmptyPort = ctx.preset !== PRESET_MAINNET
              && ctx.nodeType !== NODE_TYPE_FULLNODE;

            let form;
            if (ctx.initialIpForm) {
              form = ctx.initialIpForm;
            } else {
              form = await task.prompt(await createIpAndPortsForm(ctx.preset, {
                isHPMN: ctx.isHP,
                initialIp: '',
                initialCoreP2PPort: showEmptyPort ? '' : undefined,
                initialPlatformHTTPPort: showEmptyPort ? '' : undefined,
                initialPlatformP2PPort: showEmptyPort ? '' : undefined,
              }));
            }

            ctx.config.set('externalIp', form.ip);
            ctx.config.set('core.p2p.port', form.coreP2PPort);

            if (ctx.isHP) {
              ctx.config.set('platform.dapi.envoy.http.port', form.platformHTTPPort);
              ctx.config.set('platform.drive.tenderdash.p2p.port', form.platformP2PPort);
            }
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

  return configureNodeTask;
}

module.exports = configureNodeTaskFactory;
