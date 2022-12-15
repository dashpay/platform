/* eslint-disable quote-props */
const { Flags } = require('@oclif/core');
const { OUTPUT_FORMATS } = require('../../constants');

const GroupBaseCommand = require('../../oclif/command/GroupBaseCommand');
const printObject = require('../../printers/printObject');
const colors = require('../../status/colors');
const ServiceStatusEnum = require('../../enums/serviceStatus');

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
        // eslint-disable-next-line no-console
        console.error(scopeResult.reason);

        continue;
      }

      const { name, scope } = scopeResult.value;

      if (flags.format === OUTPUT_FORMATS.PLAIN) {
        // eslint-disable-next-line no-console
        console.log(`Node ${name}`);

        const plain = {
          'Network': scope.core.network,
          'Core Status': colors.status(scope.core.serviceStatus)(scope.core.serviceStatus),
          'Core Height': scope.core.blockHeight,
          'Platform Enabled': scope.platform.enabled,
        };

        if (scope.platform.enabled) {
          if (scope.platform.tenderdash.serviceStatus === ServiceStatusEnum.error) {
            plain['Platform Container'] = scope.platform.tenderdash.dockerStatus;
            plain['Platform Status'] = colors.status(scope.platform.tenderdash.serviceStatus)(scope.platform.tenderdash.serviceStatus);
          } else {
            plain['Platform Status'] = colors.status(scope.platform.tenderdash.serviceStatus)(scope.platform.tenderdash.serviceStatus);
            plain['Platform Version'] = scope.platform.tenderdash.version;
            plain['Platform Block Height'] = scope.platform.tenderdash.lastBlockHeight;
            plain['Platform Peers'] = scope.platform.tenderdash.peers;
            plain['Platform Network'] = scope.platform.tenderdash.network;
          }
        }

        printObject(plain, flags.format);
      } else {
        json.push({
          configName: name,
          network: scope.core.network,
          core: {
            status: scope.core.status,
            blockHeight: scope.core.blockHeight,
          },
          platform: {
            status: scope.platform.tenderdash.serviceStatus,
            version: scope.platform.tenderdash.version,
            blockHeight: scope.platform.tenderdash.block,
            peers: scope.platform.tenderdash.peers,
            network: scope.platform.tenderdash.network,
          },
        });
      }
    }

    if (flags.format === OUTPUT_FORMATS.JSON) {
      printObject(json, OUTPUT_FORMATS.JSON);
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
