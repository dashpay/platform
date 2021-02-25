const { Listr } = require('listr2');

const publicIp = require('public-ip');

const { PrivateKey: BlsPrivateKey } = require('bls-signatures');

const {
  NODE_TYPES,
  NODE_TYPE_MASTERNODE,
} = require('../../../constants');

/**
 * @param {ConfigFile} configFile
 * @param {generateBlsKeys} generateBlsKeys
 * @param {tenderdashInitTask} tenderdashInitTask
 */
function setupRegularPresetTaskFactory(
  configFile,
  generateBlsKeys,
  tenderdashInitTask,
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
        title: 'Set node type',
        task: async (ctx, task) => {
          if (ctx.nodeType === undefined) {
            ctx.nodeType = await task.prompt([
              {
                type: 'select',
                message: 'Select node type',
                choices: NODE_TYPES,
                initial: NODE_TYPE_MASTERNODE,
              },
            ]);
          }

          ctx.config.set('core.masternode.enable', ctx.nodeType === NODE_TYPE_MASTERNODE);

          // eslint-disable-next-line no-param-reassign
          task.output = `Selected ${ctx.nodeType} type\n`;
        },
        options: { persistentOutput: true },
      },
      {
        title: 'Configure external IP address',
        task: async (ctx, task) => {
          if (ctx.externalIp === undefined) {
            ctx.externalIp = await task.prompt([
              {
                type: 'input',
                message: 'Enter node public IP (Enter to accept detected IP)',
                initial: () => publicIp.v4(),
              },
            ]);
          }

          ctx.config.set('externalIp', ctx.externalIp);

          // eslint-disable-next-line no-param-reassign
          task.output = `${ctx.externalIp} is set\n`;
        },
        options: { persistentOutput: true },
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
          const privateKey = BlsPrivateKey.fromBytes(operatorBlsPrivateKeyBuffer, true);
          const publicKey = privateKey.getPublicKey();
          const publicKeyHex = Buffer.from(publicKey.serialize()).toString('hex');

          ctx.config.set('core.masternode.operator.privateKey', ctx.operatorBlsPrivateKey);

          // eslint-disable-next-line no-param-reassign
          task.output = `BLS public key: ${publicKeyHex}\nBLS private key: ${ctx.operatorBlsPrivateKey}`;
        },
        options: { persistentOutput: true },
      },
      {
        title: 'Initialize Tenderdash',
        task: (ctx) => tenderdashInitTask(ctx.config),
      },
      {
        title: 'Set default config',
        task: (ctx, task) => {
          configFile.setDefaultConfigName(ctx.preset);

          // eslint-disable-next-line no-param-reassign
          task.output = `${ctx.config.getName()} set as default config\n`;
        },
      },
    ]);
  }

  return setupRegularPresetTask;
}

module.exports = setupRegularPresetTaskFactory;
