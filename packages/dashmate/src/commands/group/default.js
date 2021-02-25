const BaseCommand = require('../../oclif/command/BaseCommand');

class GroupDefaultCommand extends BaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {ConfigFile} configFile
   * @return {Promise<void>}
   */
  async runWithDependencies(
    {
      group: groupName,
    },
    flags,
    configFile,
  ) {
    if (groupName === null) {
      // eslint-disable-next-line no-console
      console.log(configFile.getDefaultGroupName());
    } else {
      configFile.setDefaultGroupName(groupName);

      // eslint-disable-next-line no-console
      console.log(`${groupName} group set as default`);
    }
  }
}

GroupDefaultCommand.description = `Manage default group

Shows default group name or sets another group as default
`;

GroupDefaultCommand.args = [{
  name: 'group',
  required: false,
  description: 'group name',
  default: null,
}];

module.exports = GroupDefaultCommand;
