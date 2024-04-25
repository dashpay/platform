import { Flags } from '@oclif/core';
import { OUTPUT_FORMATS } from '../../constants.js';
import ConfigBaseCommand from '../../oclif/command/ConfigBaseCommand.js';

export default class ConfigRenderCommand extends ConfigBaseCommand {
  static description = `Render config's service configs

Force dashmate to render all config's service configs
`;

  static flags = {
    format: Flags.string({
      description: 'display output format',
      default: OUTPUT_FORMATS.PLAIN,
      options: Object.values(OUTPUT_FORMATS),
    }),
    ...ConfigBaseCommand.flags,
  };

  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {Config} config
   * @param {renderServiceTemplates} renderServiceTemplates
   * @param {writeServiceConfigs} writeServiceConfigs
   * @return {Promise<void>}
   */
  async runWithDependencies(args, flags, config, renderServiceTemplates, writeServiceConfigs) {
    // render & write service config files
    const configFiles = renderServiceTemplates(config);
    writeServiceConfigs(config.getName(), configFiles);

    // eslint-disable-next-line no-console
    console.log(`"${config.getName()}" service configs rendered`);
  }
}
