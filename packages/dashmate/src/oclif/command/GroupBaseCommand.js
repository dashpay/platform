const { flags: flagTypes } = require('@oclif/command');

const { asValue } = require('awilix');

const BaseCommand = require('./BaseCommand');
const GroupIsNotPresentError = require('../../config/errors/GroupIsNotPresentError');

/**
 * @abstract
 */
class GroupBaseCommand extends BaseCommand {
  async run() {
    const configFile = this.container.resolve('configFile');

    let groupName;
    if (this.parsedFlags.group !== null) {
      if (!configFile.isGroupExists(this.parsedFlags.group)) {
        throw new GroupIsNotPresentError(this.parsedFlags.group);
      }

      groupName = this.parsedFlags.group;
    } else {
      const defaultGroupName = configFile.getDefaultGroupName();

      if (defaultGroupName === null) {
        throw new Error('Default group is not set. Please use `--group` option or set default group');
      }

      if (!configFile.isGroupExists(defaultGroupName)) {
        throw new Error(`Default group ${defaultGroupName} is not exist. Please use '--group' option or change default group`);
      }

      groupName = defaultGroupName;
    }

    const group = configFile.getGroupConfigs(groupName);

    this.container.register({
      configGroup: asValue(group),
    });

    const renderServiceTemplates = this.container.resolve('renderServiceTemplates');
    const writeServiceConfigs = this.container.resolve('writeServiceConfigs');

    for (const config of group) {
      const serviceConfigFiles = renderServiceTemplates(config);
      writeServiceConfigs(config.getName(), serviceConfigFiles);
    }

    return super.run();
  }
}

GroupBaseCommand.flags = {
  group: flagTypes.string({
    description: 'group name to use',
    default: null,
  }),
  ...BaseCommand.flags,
};

module.exports = GroupBaseCommand;
