const { Flags } = require('@oclif/core');
const { OUTPUT_FORMATS } = require('../../constants');

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

      await outputStatusOverview(config, flags.format);
    }
  }
}

GroupStatusCommand.description = 'Show group status overview';

GroupStatusCommand.flags = {
  ...GroupBaseCommand.flags,
  format: Flags.string({
    description: 'display output format',
    default: OUTPUT_FORMATS.PLAIN,
    options: Object.values(OUTPUT_FORMATS),
  }),
};

module.exports = GroupStatusCommand;
