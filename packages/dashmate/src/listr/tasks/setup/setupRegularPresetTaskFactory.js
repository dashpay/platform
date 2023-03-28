const { Listr } = require('listr2');

const chalk = require('chalk');

const {
  NODE_TYPE_MASTERNODE,
  NODE_TYPE_HPMN,
  NODE_TYPE_FULLNODE,
} = require('../../../constants');

const systemConfigs = require('../../../../configs/system');

const {
  NODE_TYPE_NAMES,
  getNodeTypeByName,
  isNodeTypeNameHighPerformance,
} = require('./nodeTypes');

const Config = require('../../../config/Config');

/**
 * @param {ConfigFile} configFile
 * @param {generateBlsKeys} generateBlsKeys
 * @param {registerMasternodeTask} registerMasternodeTask
 * @param {renderServiceTemplates} renderServiceTemplates
 * @param {writeServiceConfigs} writeServiceConfigs
 * @param {obtainZeroSSLCertificateTask} obtainZeroSSLCertificateTask
 * @param {saveCertificateTask} saveCertificateTask
 * @param {listCertificates} listCertificates
 * @param {registerMasternodeGuideTask} registerMasternodeGuideTask
 * @param {configureNodeTask} configureNodeTask
 * @param {configureSSLCertificateTask} configureSSLCertificateTask
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
  registerMasternodeGuideTask,
  configureNodeTask,
  configureSSLCertificateTask,
) {
  /**
   * @typedef {setupRegularPresetTask}
   * @return {Listr}
   */
  function setupRegularPresetTask() {
    return new Listr([
      {
        title: 'Node type',
        task: async (ctx, task) => {
          const nodeTypeName = await task.prompt([
            {
              type: 'select',
              // Keep this order, because each item references the text in the previous item
              header: `  The Dash network consists of several different node types:
    Full node                   - Host the full Dash blockchain (no collateral)
    Masternode                  - Full node features, plus Core services such as
                                  ChainLocks and InstantSend (1000 DASH collateral)
    High-performance full node  - Full node features, plus host a full copy of the
                                  Platform blockchain (no collateral)
    High-performance masternode - Masternode features, plus Platform services such
                                  as DAPI and Drive (4000 DASH collateral)\n`,
              message: 'Select node type',
              choices: [
                { name: NODE_TYPE_NAMES.FULLNODE },
                { name: NODE_TYPE_NAMES.MASTERNODE, hint: '1000 DASH collateral' },
                { name: NODE_TYPE_NAMES.HP_FULLNODE },
                { name: NODE_TYPE_NAMES.HP_MASTERNODE, hint: '4000 DASH collateral' },
              ],
              initial: NODE_TYPE_NAMES.MASTERNODE,
            },
          ]);

          ctx.nodeType = getNodeTypeByName(nodeTypeName);
          ctx.isHP = isNodeTypeNameHighPerformance(nodeTypeName);

          ctx.config = new Config(ctx.preset, systemConfigs[ctx.preset]);

          if (!ctx.isHP) {
            delete ctx.config.remove('platform');
          }

          ctx.config.set('core.masternode.enable', ctx.nodeType === NODE_TYPE_MASTERNODE);

          // eslint-disable-next-line no-param-reassign
          task.output = nodeTypeName;
        },
        options: {
          persistentOutput: true,
        },
      },
      {
        enabled: (ctx) => ctx.nodeType === NODE_TYPE_MASTERNODE,
        task: async (ctx, task) => {
          let header;
          if (ctx.isHP === NODE_TYPE_HPMN) {
            header = `  If your HP masternode is already registered, we will import your masternode
  operator and platform node keys to configure an HP masternode. Please make
  sure your IP address has not changed, otherwise you will need to create a
  provider update service transaction.\n
  If you are registering a new HP masternode, dashmate will provide more
  information and help you to generate the necessary keys.\n`;
          } else {
            header = `  If your masternode is already registered, we will import your masternode
  operator key to configure a masternode. Please make sure your IP address has
  not changed, otherwise you will need to create a provider update service
  transaction.\n
  If you are registering a new masternode, dashmate will provide more
  information and help you to generate the necessary keys.\n`;
          }

          ctx.isMasternodeRegistered = await task.prompt({
            type: 'toggle',
            header,
            message: 'Is your masternode already registered?',
            enabled: 'Yes',
            disabled: 'No',
          });
        },
      },
      {
        enabled: (ctx) => !ctx.isMasternodeRegistered && ctx.nodeType === NODE_TYPE_MASTERNODE,
        task: () => registerMasternodeGuideTask(),
      },
      {
        enabled: (ctx) => ctx.isMasternodeRegistered || ctx.nodeType === NODE_TYPE_FULLNODE,
        task: () => configureNodeTask(),
      },
      {
        enabled: (ctx) => ctx.config && ctx.config.isPlatformEnabled(),
        task: () => configureSSLCertificateTask(),
      },
      {
        task: (ctx, task) => {
          configFile.setConfig(ctx.config);
          configFile.setDefaultConfigName(ctx.preset);

          // eslint-disable-next-line no-param-reassign
          task.output = chalk`Node configuration completed successfully!

            You can now run {bold.cyanBright dashmate start} to start your node, followed by
            {bold.cyanBright dashmate status} for a node health status overview.

            Run {bold.cyanBright dashmate --help} or {bold.cyanBright dashmate <command> --help} for quick help on how
            to use dashmate to manage your node.\n`;
        },
        options: {
          persistentOutput: true,
          rendererOptions: {
            bottomBar: true,
          },
        },
      },
    ]);
  }

  return setupRegularPresetTask;
}

module.exports = setupRegularPresetTaskFactory;
