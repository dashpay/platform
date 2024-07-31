import { Listr } from 'listr2';

import chalk from 'chalk';

import {
  NODE_TYPE_MASTERNODE,
  NODE_TYPE_FULLNODE,
} from '../../../constants.js';

import {
  NODE_TYPE_NAMES,
  getNodeTypeByName,
  getNodeTypeNameByType,
  isNodeTypeNameHighPerformance,
} from './nodeTypes.js';
import generateRandomString from '../../../util/generateRandomString.js';

/**
 * @param {ConfigFile} configFile
 * @param {generateBlsKeys} generateBlsKeys
 * @param {registerMasternodeTask} registerMasternodeTask
 * @param {obtainZeroSSLCertificateTask} obtainZeroSSLCertificateTask
 * @param {registerMasternodeGuideTask} registerMasternodeGuideTask
 * @param {configureNodeTask} configureNodeTask
 * @param {configureSSLCertificateTask} configureSSLCertificateTask
 * @param {DefaultConfigs} defaultConfigs
 * @param {verifySystemRequirementsTask} verifySystemRequirementsTask
 * @param {importCoreDataTask} importCoreDataTask
 */
export default function setupRegularPresetTaskFactory(
  configFile,
  generateBlsKeys,
  registerMasternodeTask,
  obtainZeroSSLCertificateTask,
  registerMasternodeGuideTask,
  configureNodeTask,
  configureSSLCertificateTask,
  defaultConfigs,
  importCoreDataTask,
  verifySystemRequirementsTask,
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
          if (!ctx.nodeType) {
            ctx.nodeTypeName = await task.prompt([
              {
                type: 'select',
                // Keep this order, because each item references the text in the previous item
                header: `  The Dash network consists of several different node types:
      Fullnode             - Host the full Dash blockchain (no collateral)
      Masternode           - Fullnode features, plus Core services such as ChainLocks
                            and InstantSend (1000 DASH collateral)
      Evolution fullnode   - Fullnode features, plus host a full copy of the Platform
                            blockchain (no collateral)
      Evolution masternode - Masternode features, plus Platform services such as DAPI
                            and Drive (4000 DASH collateral)\n`,
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

            ctx.nodeType = getNodeTypeByName(ctx.nodeTypeName);
            ctx.isHP = isNodeTypeNameHighPerformance(ctx.nodeTypeName);
          } else {
            ctx.nodeTypeName = getNodeTypeNameByType(ctx.nodeType);
          }

          ctx.config = defaultConfigs.get(ctx.preset);

          ctx.config.set('platform.enable', ctx.isHP);
          ctx.config.set('core.masternode.enable', ctx.nodeType === NODE_TYPE_MASTERNODE);

          if (ctx.config.get('core.masternode.enable')) {
            ctx.config.set('platform.drive.tenderdash.mode', 'validator');
          } else {
            ctx.config.set('platform.drive.tenderdash.mode', 'full');
          }

          Object.values(ctx.config.get('core.rpc.users')).forEach((options) => {
            // eslint-disable-next-line no-param-reassign
            options.password = generateRandomString(12);
          });

          // eslint-disable-next-line no-param-reassign
          task.output = ctx.nodeTypeName;
        },
        options: {
          persistentOutput: true,
        },
      },
      {
        task: () => verifySystemRequirementsTask(),
      },
      {
        enabled: (ctx) => ctx.nodeType === NODE_TYPE_MASTERNODE,
        task: async (ctx, task) => {
          let header;
          if (ctx.isHP) {
            header = `  If your Evo masternode is already registered, we will import your masternode
  operator and platform node keys to configure an Evo masternode. Please make
  sure your IP address has not changed, otherwise you will need to create a
  provider update service transaction.\n
  If you are registering a new Evo masternode, dashmate will provide more
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
        enabled: (ctx) => ctx.isMasternodeRegistered,
        task: () => importCoreDataTask(),
      },
      {
        enabled: (ctx) => ctx.isMasternodeRegistered || ctx.nodeType === NODE_TYPE_FULLNODE,
        task: () => configureNodeTask(),
      },
      {
        enabled: (ctx) => ctx.config && ctx.config.get('platform.enable'),
        task: () => configureSSLCertificateTask(),
      },
      {
        task: (ctx, task) => {
          configFile.setConfig(ctx.config);
          configFile.setDefaultConfigName(ctx.preset);

          let startInstructions = '';
          if (ctx.isReindexRequired) {
            startInstructions = chalk`You existing Core node doesn't have indexes required to run ${ctx.nodeTypeName}
            Please run {bold.cyanBright dashmate core reindex} to reindex your node.
            The node will be started automatically after reindex is complete.`;
          } else {
            startInstructions = chalk`You can now run {bold.cyanBright dashmate start} to start your node, followed by
            {bold.cyanBright dashmate status} for a node health status overview.`;
          }

          // eslint-disable-next-line no-param-reassign
          task.output = chalk`Node configuration completed successfully!

            ${startInstructions}

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
