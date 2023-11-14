import { BaseCommand } from '../../oclif/command/BaseCommand.js';

export class GroupDefaultCommand extends BaseCommand {
  static description = `Manage default group

Shows default group name or sets another group as default
`;

  static args = [{
    name: 'group',
    required: false,
    description: 'group name',
    default: null,
  }]

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
