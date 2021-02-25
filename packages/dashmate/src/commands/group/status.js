const GroupBaseCommand = require('../../oclif/command/GroupBaseCommand');

class GroupStatusCommand extends GroupBaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {outputStatusOverview} outputStatusOverview
   * @param {Config[]} configGroup
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    flags,
    outputStatusOverview,
    configGroup,
  ) {
    for (const config of configGroup) {
      // eslint-disable-next-line no-console
      console.log(`Node ${config.getName()}`);

      await outputStatusOverview(config);
    }
  }
}

GroupStatusCommand.description = 'Show group status overview';

GroupStatusCommand.flags = {
  ...GroupBaseCommand.flags,
};

module.exports = GroupStatusCommand;
