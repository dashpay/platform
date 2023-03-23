const { Listr } = require('listr2');

const chalk = require('chalk');

const BlsSignatures = require('@dashevo/bls');

const {
  NODE_TYPE_MASTERNODE,
  PRESET_MAINNET,
} = require('../../../../constants');

const validateBLSPrivateKeyFactory = require('../../../prompts/validators/validateBLSPrivateKeyFactory');
const createPlatformNodeKeyInput = require('../../../prompts/createPlatformNodeKeyInput');
const createIpAndPortsForm = require('../../../prompts/createIpAndPortsForm');
const createTenderdashNodeId = require('../../../../tenderdash/createTenderdashNodeId');

function configureNodeTaskFactory() {
  /**
   * @typedef configureNodeTask
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
              header: `  Please enter your existing BLS private key.

  Your BLS private key can be found in the "dash.conf" file of your masternode,
  in the DMT configuration tool, or in the safe location in which you stored it
  when initially configuring your masternode.\n`,
              message: 'BLS private key',
              hint: 'HEX encoded',
              validate: validateBLSPrivateKey,
            });

            ctx.config.set('core.masternode.operator.privateKey', masternodeOperatorPrivateKey);
          }

          // Platform Node Key
          if (ctx.isHP) {
            const platformNodeKey = await task.prompt(createPlatformNodeKeyInput({
              skipInitial: ctx.nodeType === NODE_TYPE_MASTERNODE,
            }));

            ctx.config.set('platform.drive.tenderdash.node.id', createTenderdashNodeId(platformNodeKey));
            ctx.config.set('platform.drive.tenderdash.node.key', platformNodeKey);
          }

          // IP and ports
          if (ctx.nodeType === NODE_TYPE_MASTERNODE) {
            const form = await task.prompt(await createIpAndPortsForm({
              isHPMN: ctx.isHP,
              skipInitial: true,
            }));

            ctx.config.set('externalIp', form.ip);

            if (ctx.preset !== PRESET_MAINNET) {
              ctx.config.set('core.p2p.port', form.coreP2PPort);

              if (ctx.isHP) {
                ctx.config.set('platform.dapi.envoy.http.port', form.platformHTTPPort);
                ctx.config.set('platform.drive.tenderdash.p2p.port', form.platformP2PPort);
              }
            }
          }

          // TODO: Output configuration
        },
      },
    ]);
  }

  return configureNodeTask;
}

module.exports = configureNodeTaskFactory;
