/* eslint-disable quote-props */
import { Flags } from '@oclif/core';
import GroupBaseCommand from '../../oclif/command/GroupBaseCommand.js';
import printArrayOfObjects from '../../printers/printArrayOfObjects.js';
import printObject from '../../printers/printObject.js';
import { OUTPUT_FORMATS } from '../../constants.js';
import colors from '../../status/colors.js';
import { ServiceStatusEnum } from '../../status/enums/serviceStatus.js';

export default class GroupStatusCommand extends GroupBaseCommand {
  static description = 'Show group status overview';

  static flags = {
    ...GroupBaseCommand.flags,
    format: Flags.string({
      description: 'display output format',
      default: OUTPUT_FORMATS.PLAIN,
      options: Object.values(OUTPUT_FORMATS),
    }),
  };

  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {getOverviewScope} getOverviewScope
   * @param {Config[]} configGroup
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    flags,
    getOverviewScope,
    configGroup,
  ) {
    const scopeResults = await Promise.allSettled(configGroup.map(async (config) => {
      // try-catch needed to pass config name in the error
      try {
        const { name } = config;
        const scope = await getOverviewScope(config);

        return { name, scope };
      } catch (e) {
        throw new Error(`Could not retrieve data for node group ${config.name}, reason: ${e}`);
      }
    }));

    const json = [];

    for (const scopeResult of scopeResults) {
      if (scopeResult.status !== 'fulfilled') {
        if (process.env.DEBUG) {
          // eslint-disable-next-line no-console
          console.error(scopeResult.reason);
        }

        continue;
      }

      const plain = {
        'Network': 'n/a',
        'Core Status': 'n/a',
        'Core Height': 'n/a',
        'Platform Enabled': 'n/a',
        'Platform Container': 'n/a',
        'Platform Status': 'n/a',
        'Platform Version': 'n/a',
        'Platform Block Height': 'n/a',
        'Platform Peers': 'n/a',
        'Platform Network': 'n/a',
      };

      const { name, scope } = scopeResult.value;

      if (flags.format === OUTPUT_FORMATS.PLAIN) {
        // eslint-disable-next-line no-console
        console.log(`Node ${name}`);

        plain.Network = scope.core.network || 'n/a';
        plain['Core Status'] = colors.status(scope.core.serviceStatus)(scope.core.serviceStatus) || 'n/a';
        plain['Core Height'] = scope.core.blockHeight || 'n/a';
        plain['Platform Enabled'] = scope.platform.enabled || 'n/a';

        if (scope.platform.enabled) {
          if (scope.platform.tenderdash.serviceStatus === ServiceStatusEnum.error) {
            plain['Platform Container'] = scope.platform.tenderdash.dockerStatus || 'n/a';
            plain['Platform Status'] = colors.status(scope.platform.tenderdash.serviceStatus)(scope.platform.tenderdash.serviceStatus) || 'n/a';
          } else {
            plain['Platform Status'] = colors.status(scope.platform.tenderdash.serviceStatus)(scope.platform.tenderdash.serviceStatus) || 'n/a';
            plain['Platform Version'] = scope.platform.tenderdash.version || 'n/a';
            plain['Platform Block Height'] = scope.platform.tenderdash.latestBlockHeight || 'n/a';
            plain['Platform Peers'] = scope.platform.tenderdash.peers || 'n/a';
            plain['Platform Network'] = scope.platform.tenderdash.network || 'n/a';
          }
        }

        printObject(plain, flags.format);
      } else {
        json.push(scope);
      }
    }

    if (flags.format === OUTPUT_FORMATS.JSON) {
      printArrayOfObjects(json, OUTPUT_FORMATS.JSON);
    }
  }
}
