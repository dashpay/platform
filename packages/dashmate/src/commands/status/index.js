const { Flags } = require('@oclif/core');
const { OUTPUT_FORMATS } = require('../../constants');

const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');
const printObject = require('../../printers/printObject');
const MasternodeStateEnum = require('../../status/enums/masternodeState');
const colors = require('../../status/colors');
const ServiceStatusEnum = require('../../status/enums/serviceStatus');

class StatusCommand extends ConfigBaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {dockerCompose} dockerCompose
   * @param {getOverviewScope} getOverviewScope
   * @param {Config} config
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    flags,
    dockerCompose,
    getOverviewScope,
    config,
  ) {
    const plain = {
      Network: 'n/a',
      'Core Version': 'n/a',
      'Core Status': 'n/a',
      'Core Service Status': 'n/a',
      'Core Size': 'n/a',
      'Core Height': 'n/a',
      'Core Sync Progress': 'n/a',
      'Masternode Enabled': 'n/a',
      'Masternode State': 'n/a',
      'Masternode ProTX': 'n/a',
      'PoSe Penalty': 'n/a',
      'Last paid block': 'n/a',
      'Last paid time': 'n/a',
      'Payment queue position': 'n/a',
      'Next payment time': 'n/a',
      'Sentinel Version': 'n/a',
      'Sentinel Status': 'n/a',
      'Platform Enabled': 'n/a',
      'Platform Status': 'n/a',
      'Platform Version': 'n/a',
      'Platform Block Height': 'n/a',
      'Platform Peers': 'n/a',
      'Platform Network': 'n/a',
    };

    const scope = await getOverviewScope(config);

    if (flags.format === OUTPUT_FORMATS.PLAIN) {
      const {
        network, core, masternode, platform,
      } = scope;
      const {
        dockerStatus, serviceStatus, version, verificationProgress, sizeOnDisk, blockHeight,
      } = core;

      plain.Network = network || 'n/a';
      plain['Core Version'] = version || 'n/a';
      plain['Core Status'] = dockerStatus || 'n/a';
      plain['Core Service Status'] = colors.status(serviceStatus)(serviceStatus) || 'n/a';
      plain['Core Size'] = sizeOnDisk ? `${(sizeOnDisk / 1024 / 1024 / 1024).toFixed(2)} GB` : 'n/a';
      plain['Core Height'] = blockHeight || 'n/a';

      if (serviceStatus === ServiceStatusEnum.syncing) {
        plain['Core Sync Progress'] = verificationProgress ? `${(verificationProgress * 100).toFixed(2)}%` : 'n/a';
      }

      plain['Masternode Enabled'] = masternode.enabled || 'n/a';

      if (masternode.enabled) {
        plain['Masternode State'] = masternode.state || 'n/a';

        if (masternode.state === MasternodeStateEnum.READY) {
          const {
            enabledCount,
            poSePenalty,
            lastPaidHeight,
            lastPaidTime,
            paymentQueuePosition,
            nextPaymentTime,
          } = masternode.nodeState;

          plain['Masternode ProTX'] = masternode.proTxHash || 'n/a';
          plain['PoSe Penalty'] = colors.poSePenalty(poSePenalty, enabledCount)(`${poSePenalty}`) || 'n/a';
          plain['Last paid block'] = lastPaidHeight || 'n/a';
          plain['Last paid time'] = lastPaidHeight === 0 ? 'Never' : (lastPaidTime || 'n/a');
          plain['Payment queue position'] = paymentQueuePosition || 'n/a';
          plain['Next payment time'] = nextPaymentTime || 'n/a';
        }

        if (masternode.sentinel.version) {
          plain['Sentinel Version'] = masternode.sentinel.version || 'n/a';
          plain['Sentinel Status'] = colors.sentinel(masternode.sentinel.state)(masternode.sentinel.state) || 'n/a';
        }
      }

      plain['Platform Enabled'] = platform.enabled || 'n/a';

      if (platform.enabled) {
        plain['Platform Status'] = colors.status(platform.tenderdash.serviceStatus)(platform.tenderdash.serviceStatus) || 'n/a';

        if (platform.tenderdash.serviceStatus === ServiceStatusEnum.up) {
          plain['Platform Version'] = platform.tenderdash.version || 'n/a';
          plain['Platform Block Height'] = platform.tenderdash.latestBlockHeight || 'n/a';
          plain['Platform Peers'] = platform.tenderdash.peers || 'n/a';
          plain['Platform Network'] = platform.tenderdash.network || 'n/a';
        }
      }

      return printObject(plain, flags.format);
    }

    return printObject(scope, flags.format);
  }
}

StatusCommand.description = 'Show status overview';

StatusCommand.flags = {
  ...ConfigBaseCommand.flags,
  format: Flags.string({
    description: 'display output format',
    default: OUTPUT_FORMATS.PLAIN,
    options: Object.values(OUTPUT_FORMATS),
  }),
};

module.exports = StatusCommand;
