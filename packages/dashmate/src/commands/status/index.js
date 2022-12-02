const chalk = require('chalk')
const {Flags} = require('@oclif/core');
const {OUTPUT_FORMATS} = require('../../constants');

const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');
const printObject = require("../../printers/printObject");

class StatusCommand extends ConfigBaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {outputStatusOverview} outputStatusOverview
   * @param {Config} config
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    flags,
    outputStatusOverview,
    config,
  ) {
    const statusOverview = await outputStatusOverview(config, ['core']);

    const network = config.get('network')
    const masternodeEnabled = config.get('core.masternode.enable')

    const {core} = statusOverview
    const {
      version: coreVersion,
      status: coreStatus,
      isSynced,
      verificationProgress
    } = core

    const json = {
      network,
      coreVersion,
      coreStatus,
      platform: null,
      masternode: {
        enabled: masternodeEnabled,
        status: null,
        state: {
          poSePenalty: null,
          lastPaidHeight: null,
          lastPaidTime: null,
          paymentQueuePosition: null,
          nextPaymentTime: null
        }
      }
    }

    if (masternodeEnabled) {
      const {masternode: masternodeStatus, state} = await outputStatusOverview(config, ['masternode'])

      json.masternode.status = masternodeStatus
      json.masternode.state = state
    }

    if (config.get('network') !== 'mainnet' && config.name !== 'local_seed') {
      const {platform} = await outputStatusOverview(config, ['platform'])
      const {status: platformStatus} = platform

      json.platform = {
        status: platformStatus,
        version: null
      }

      if (isSynced === true && platformStatus !== 'not_started' && platformStatus !== 'restarting') {
        const {tenderdash} = platform
        const {version: tenderdashVersion} = tenderdash

        json.platform.version = tenderdashVersion;
      }
    }

    if (flags.format === OUTPUT_FORMATS.PLAIN) {
      const plain = {
        'Network': network,
        'Core Version': coreVersion,
        'Core Status': coreStatus,
      }

      if (coreStatus === 'syncing') {
        plain['Core Sync Progress'] = `${verificationProgress * 100}%`
      }

      // Colors
      switch (coreStatus) {
        case 'running':
          plain["Core Status"] = chalk.green(plain["Core Status"])
          break;
        case 'syncing':
          plain["Core Status"] = chalk.yellow(plain["Core Status"])
          break;
        default:
          plain["Core Status"] = chalk.red(plain["Core Status"])
      }


      if (json.masternode.enabled) {
        if (json.masternode.status === 'Ready') {
          plain["Masternode Status"] = chalk.green(json.masternode.status);

          const {
            PoSePenalty,
            lastPaidHeight,
            lastPaidTime,
            paymentQueuePosition,
            nextPaymentTime
          } = json.masternode.state

          plain['PoSe Penalty'] = PoSePenalty;
          plain['Last paid block'] = lastPaidHeight;
          plain['Last paid time'] = lastPaidTime
          plain['Payment queue position'] = paymentQueuePosition;
          plain['Next payment time'] = nextPaymentTime;
        } else {
          plain["Masternode Status"] = chalk.red(json.masternode.status);
        }
      }

      if (json.platform) {
        //todo syncing
        if (json.platform.status === 'running') {
          plain['Platform Status'] = chalk.green(json.platform.status);
        } else {
          plain['Platform Status'] = chalk.red(json.platform.status);
        }

        if (json.platform.status.version) {
          plain['Platform Version'] = json.platform.version;
        }
      }

      return printObject(plain, flags.format);
    }

    printObject(json, flags.format);
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
