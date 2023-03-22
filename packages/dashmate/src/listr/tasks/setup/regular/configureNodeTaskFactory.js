const { Listr } = require('listr2');

const chalk = require('chalk');

const BlsSignatures = require('@dashevo/bls');

const {
  NODE_TYPE_MASTERNODE,
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

          if (ctx.nodeType === NODE_TYPE_MASTERNODE) {
            const masternodeOperatorPrivateKey = await task.prompt({
              type: 'input',
              header: 'Any suggestions where to get it?\n',
              message: 'BLS private key',
              hint: 'HEX encoded',
              validate: validateBLSPrivateKey,
            });

            ctx.config.set('core.masternode.operator.privateKey', masternodeOperatorPrivateKey);
          }

          if (ctx.isHP) {
            const platformNodeKey = await task.prompt(createPlatformNodeKeyInput({
              skipInitial: ctx.nodeType === NODE_TYPE_MASTERNODE,
            }));

            ctx.config.set('platform.drive.tenderdash.node.id', createTenderdashNodeId(platformNodeKey));
            ctx.config.set('platform.drive.tenderdash.node.key', platformNodeKey);
          }

          const form = await task.prompt(await createIpAndPortsForm({
            isHPMN: ctx.isHP,
            skipInitial: ctx.nodeType === NODE_TYPE_MASTERNODE,
          }));

          ctx.config.set('core.p2p.port', form.coreP2PPort);
          ctx.config.set('externalIp', form.ip);

          if (ctx.isHP) {
            ctx.config.set('platform.dapi.envoy.http.port', form.platformHTTPPort);
            ctx.config.set('platform.drive.tenderdash.p2p.port', form.platformP2PPort);
          }
        },
      },
    ]);
  }

  return configureNodeTask;
}

module.exports = configureNodeTaskFactory;
