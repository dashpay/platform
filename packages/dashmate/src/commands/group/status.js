/* eslint-disable quote-props */
const { Flags } = require('@oclif/core');
const { OUTPUT_FORMATS } = require('../../constants');

const GroupBaseCommand = require('../../oclif/command/GroupBaseCommand');
const printObject = require('../../printers/printObject');
const printArrayOfObjects = require('../../printers/printArrayOfObjects');
const colors = require('../../status/colors');
const ServiceStatusEnum = require('../../status/enums/serviceStatus');

class GroupStatusCommand extends GroupBaseCommand {
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

        plain.Network = scope.core.network;
        plain['Core Status'] = colors.status(scope.core.serviceStatus)(scope.core.serviceStatus);
        plain['Core Height'] = scope.core.blockHeight;
        plain['Platform Enabled'] = scope.platform.enabled;

        if (scope.platform.enabled) {
          if (scope.platform.tenderdash.serviceStatus === ServiceStatusEnum.error) {
            plain['Platform Container'] = scope.platform.tenderdash.dockerStatus;
            plain['Platform Status'] = colors.status(scope.platform.tenderdash.serviceStatus)(scope.platform.tenderdash.serviceStatus);
          } else {
            plain['Platform Status'] = colors.status(scope.platform.tenderdash.serviceStatus)(scope.platform.tenderdash.serviceStatus);
            plain['Platform Version'] = scope.platform.tenderdash.version;
            plain['Platform Block Height'] = scope.platform.tenderdash.latestBlockHeight;
            plain['Platform Peers'] = scope.platform.tenderdash.peers;
            plain['Platform Network'] = scope.platform.tenderdash.network;
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
