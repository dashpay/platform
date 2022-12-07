const {Flags} = require('@oclif/core');
const {OUTPUT_FORMATS} = require('../../constants');

const GroupBaseCommand = require('../../oclif/command/GroupBaseCommand');
const printObject = require("../../printers/printObject");
const colors = require("../../status/colors");
const MasternodeStateEnum = require("../../enums/masternodeState");
const ServiceStatusEnum = require("../../enums/serviceStatus");
const {platform} = require("../../status/scopes");

class GroupStatusCommand extends GroupBaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {statusProvider} statusProvider
   * @param {Config[]} configGroup
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    flags,
    statusProvider,
    configGroup,
  ) {

    const status = []

    for (const config of configGroup) {
      // eslint-disable-next-line no-console
      console.log(`Node ${config.getName()}`);

      const scope = await statusProvider.getOverviewScope(config);

      if (flags.format === OUTPUT_FORMATS.PLAIN) {
        const plain = {
          'Network': config.get('network'),
          'Core Status': colors.status(scope.core.serviceStatus)(scope.core.serviceStatus),
          'Core Height': scope.core.blockHeight,
          'Platform Enabled': scope.platform.enabled
        };

        if (scope.platform.enabled) {
          if (scope.platform.tenderdash.serviceStatus === ServiceStatusEnum.error) {
            plain['Platform Container'] = scope.platform.tenderdash.dockerStatus
            plain['Platform Status'] = scope.platform.tenderdash.serviceStatus
          } else {
            plain['Platform Status'] = scope.platform.tenderdash.serviceStatus
            plain['Platform Version'] = scope.platform.tenderdash.version
            plain['Platform Block Height'] = scope.platform.tenderdash.lastBlockHeight
            plain['Platform Peers'] = scope.platform.tenderdash.peers
            plain['Platform Network'] = scope.platform.tenderdash.network
          }
        }

        printObject(plain, flags.format);
      } else {
        status.push({
          network: config.get('network'),
          core: {
            status: scope.core.status,
            blockHeight: scope.core.blockHeight
          },
          platform: {
            status: scope.platform.tenderdash.serviceStatus,
            version: scope.platform.tenderdash.version,
            blockHeight: scope.platform.tenderdash.block,
            peers: scope.platform.tenderdash.peers,
            network: scope.platform.tenderdash.network
          }
        })
      }
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
