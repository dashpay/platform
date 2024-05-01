import { Flags } from '@oclif/core';
import { asValue } from 'awilix';
import BaseCommand from './BaseCommand.js';
import GroupIsNotPresentError from '../../config/errors/GroupIsNotPresentError.js';

/**
 * @abstract
 */
export default class GroupBaseCommand extends BaseCommand {
  static flags = {
    group: Flags.string({
      description: 'group name to use',
      default: null,
    }),
    ...BaseCommand.flags,
  };

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
        throw new Error(`Default group ${defaultGroupName} does not exist. Please use '--group' option or change default group`);
      }

      groupName = defaultGroupName;
    }

    const group = configFile.getGroupConfigs(groupName);

    this.container.register({
      configGroup: asValue(group),
    });

    return super.run();
  }
}
