import { table } from 'table';
import { BaseCommand } from '../../oclif/command/BaseCommand.js';

export class ConfigListCommand extends BaseCommand {
  static description = 'List available configs';

  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {ConfigFile} configFile
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    flags,
    configFile,
  ) {
    const rows = configFile.getAllConfigs()
      .map((config) => [config.getName(), config.get('description')]);

    const output = table(rows);

    // eslint-disable-next-line no-console
    console.log(output);
  }
}
