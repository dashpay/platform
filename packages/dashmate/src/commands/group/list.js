import { table } from 'table';
import GroupBaseCommand from '../../oclif/command/GroupBaseCommand.js';

export default class GroupListCommand extends GroupBaseCommand {
  static description = 'List available groups';

  static flags = {
    ...GroupBaseCommand.flags,
  };

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
