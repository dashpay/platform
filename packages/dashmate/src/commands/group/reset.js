const { Listr } = require('listr2');

const { flags: flagTypes } = require('@oclif/command');

const baseConfig = require('../../../configs/system/base');

const GroupBaseCommand = require('../../oclif/command/GroupBaseCommand');
const MuteOneLineError = require('../../oclif/errors/MuteOneLineError');

class GroupResetCommand extends GroupBaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {isSystemConfig} isSystemConfig
   * @param {resetNodeTask} resetNodeTask
   * @param {Config[]} configGroup
   * @param {configureCoreTask} configureCoreTask
   * @param {configureTenderdashTask} configureTenderdashTask
   * @param {initializePlatformTask} initializePlatformTask
   * @param {generateToAddressTask} generateToAddressTask
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    {
      verbose: isVerbose,
      hard: isHardReset,
      'platform-only': isPlatformOnlyReset,
    },
    isSystemConfig,
    resetNodeTask,
    configGroup,
    configureCoreTask,
    configureTenderdashTask,
    initializePlatformTask,
    generateToAddressTask,
  ) {
    const groupName = configGroup[0].get('group');

    if (isHardReset && !isSystemConfig(groupName)) {
      throw new Error(`Cannot hard reset non-system config group "${configGroup[0].get('group')}"`);
    }

    const amount = 100;

    const tasks = new Listr(
      [
        {
          title: `Reset ${groupName} nodes`,
          task: () => new Listr(configGroup.map((config) => ({
            title: `Reset ${config.getName()} node`,
            task: (ctx) => {
              ctx.skipPlatformInitialization = true;

              if (config.has('platform')) {
                config.set('platform.dpns', baseConfig.platform.dpns);
                config.set('platform.dashpay', baseConfig.platform.dashpay);

                // TODO: Should stay the same
                config.set('platform.drive.tenderdash.nodeId', baseConfig.platform.drive.tenderdash.nodeId);
                config.set('platform.drive.tenderdash.validatorKey', baseConfig.platform.drive.tenderdash.validatorKey);
                config.set('platform.drive.tenderdash.nodeKey', baseConfig.platform.drive.tenderdash.nodeKey);
                config.set('platform.drive.tenderdash.genesis', baseConfig.platform.drive.tenderdash.genesis);
              }

              if (!ctx.isPlatformOnlyReset) {
                config.set('core.masternode.operator.privateKey', baseConfig.core.masternode.operator.privateKey);
              }

              return resetNodeTask(config);
            },
          }))),
        },
        {
          enabled: (ctx) => !ctx.isHardReset,
          title: 'Configure Tenderdash nodes',
          task: () => configureTenderdashTask(configGroup),
        },
        {
          enabled: (ctx) => !ctx.isHardReset && !ctx.isPlatformOnlyReset,
          title: 'Configure Core nodes',
          task: () => configureCoreTask(configGroup),
        },
        {
          // in case we don't need to register masternodes
          title: `Generate ${amount} dash to local wallet`,
          enabled: () => !isHardReset,
          skip: (ctx) => !!ctx.fundingPrivateKeyString,
          task: () => generateToAddressTask(configGroup[0], amount),
        },
        {
          enabled: (ctx) => !ctx.isHardReset,
          title: 'Initialize Platform',
          task: () => initializePlatformTask(configGroup),
        },
      ],
      {
        renderer: isVerbose ? 'verbose' : 'default',
        rendererOptions: {
          clearOutput: false,
          collapse: false,
          showSubtasks: true,
        },
      },
    );

    try {
      await tasks.run({
        isHardReset,
        isPlatformOnlyReset,
      });
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}

GroupResetCommand.description = 'Reset group nodes';

GroupResetCommand.flags = {
  ...GroupBaseCommand.flags,
  hard: flagTypes.boolean({ char: 'h', description: 'reset config as well as data', default: false }),
  'platform-only': flagTypes.boolean({ char: 'p', description: 'reset platform data only', default: false }),
};

module.exports = GroupResetCommand;
