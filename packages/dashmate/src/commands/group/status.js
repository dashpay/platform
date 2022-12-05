const {Flags} = require('@oclif/core');
const {OUTPUT_FORMATS} = require('../../constants');

const GroupBaseCommand = require('../../oclif/command/GroupBaseCommand');
const printObject = require("../../printers/printObject");
const colors = require("../../status/colors");
const MasternodeStateEnum = require("../../enums/masternodeState");

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
          'Core Status': colors.status(scope.core.status)(scope.core.status),
          'Core Height': scope.core.blockHeight,
          'Platform Enabled': scope.platform.enabled,
          'Platform Status': scope.platform.status,
          'Platform Version': scope.platform.tenderdash.version,
          'Platform Block Height': scope.platform.tenderdash.blockHeight,
          'Platform Peers': scope.platform.tenderdash.peers,
          'Platform Network': scope.platform.tenderdash.network
        };

        printObject(plain, flags.format);
      } else {
        status.push({
          network: config.get('network'),
          core: {
            status: scope.core.status,
            blockHeight: scope.core.blockHeight
          },
          platform: {
            status: scope.platform.status,
            version: scope.platform.tenderdash.version,
            blockHeight: scope.platform.tenderdash.block,
            peers: scope.platform.tenderdash.peers,
            network: scope.platform.tenderdash.network
          }
        })
      }
    }
    printObject(status, flags.format);
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
