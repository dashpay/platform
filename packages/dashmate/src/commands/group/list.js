const { table } = require('table');

const GroupBaseCommand = require('../../oclif/command/GroupBaseCommand');

class GroupListCommand extends GroupBaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {Config[]} configGroup
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    flags,
    configGroup,
  ) {
    const rows = configGroup.map((config) => [config.getName(), config.get('description')]);

    const output = table(rows);

    // eslint-disable-next-line no-console
    console.log(output);
  }
}

GroupListCommand.description = 'List available groups';

GroupListCommand.flags = {
  ...GroupBaseCommand.flags,
};

module.exports = GroupListCommand;
