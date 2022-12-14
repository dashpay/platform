const { Flags } = require('@oclif/core');
const { OUTPUT_FORMATS } = require('../../constants');

const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');
const printObject = require('../../printers/printObject');
const MasternodeStateEnum = require('../../enums/masternodeState');
const colors = require('../../status/colors');
const ServiceStatusEnum = require('../../enums/serviceStatus');

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
    if (!(await dockerCompose.isServiceRunning(config.toEnvs()))) {
      // eslint-disable-next-line no-console
      console.log('Regular node is not running! Start it with `dashmate start`');
      this.exit();
    }

    const scope = await getOverviewScope();

    if (flags.format === OUTPUT_FORMATS.PLAIN) {
      const {
        network, core, masternode, platform,
      } = scope;
      const {
        status, version, verificationProgress, sizeOnDisk, blockHeight,
      } = core;

      const plain = {
        Network: network,
        'Core Version': version,
        'Core Status': colors.status(status)(status),
        'Core Size': `${(sizeOnDisk / 1024 / 1024 / 1024).toFixed(2)} GB`,
        'Core Height': blockHeight,
      };

      if (status === 'syncing') {
        plain['Core Sync Progress'] = `${(verificationProgress * 100).toFixed(2)}%`;
      }

      plain['Masternode Enabled'] = masternode.enabled;

      if (masternode.enabled) {
        plain['Masternode State'] = masternode.state;

        if (masternode.state === MasternodeStateEnum.READY) {
          const {
            PoSePenalty,
            lastPaidHeight,
            lastPaidTime,
            paymentQueuePosition,
            nextPaymentTime,
          } = masternode.nodeState;

          plain['Masternode ProTX'] = masternode.proTxHash;
          plain['PoSe Penalty'] = PoSePenalty;
          plain['Last paid block'] = lastPaidHeight;
          plain['Last paid time'] = lastPaidTime;
          plain['Payment queue position'] = paymentQueuePosition;
          plain['Next payment time'] = nextPaymentTime;
        }

        plain['Sentinel Version'] = version;
        plain['Sentinel State'] = masternode;
      }

      plain['Platform Enabled'] = platform.enabled;

      if (platform.enabled) {
        plain['Platform Status'] = colors.status(platform.tenderdash.serviceStatus)(platform.serviceStatus);

        if (platform.tenderdash.serviceStatus === ServiceStatusEnum.up) {
          plain['Platform Version'] = platform.tenderdash.version;
          plain['Platform Block Height'] = platform.tenderdash.blockHeight;
          plain['Platform Peers'] = platform.tenderdash.peers;
          plain['Platform Network'] = platform.tenderdash.network;
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
