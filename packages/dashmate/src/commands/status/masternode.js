const chalk = require('chalk');

const { Flags } = require('@oclif/core');
const { OUTPUT_FORMATS } = require('../../constants');

const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');
const printObject = require('../../printers/printObject');
const colors = require('../../status/colors');
const MasternodeStateEnum = require('../../enums/masternodeState');
const MasternodeSyncAssetEnum = require('../../enums/masternodeSyncAsset');

class MasternodeStatusCommand extends ConfigBaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {DockerCompose} dockerCompose
   * @param {createRpcClient} createRpcClient
   * @param {Config} config
   * @param getMasternodeScope getMasternodeScope
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    flags,
    dockerCompose,
    createRpcClient,
    config,
    getMasternodeScope,
  ) {
    if (config.get('core.masternode.enable') === false) {
      throw new Error('This is not a masternode!');
    }

    const scope = await getMasternodeScope(config);

    if (flags.format === OUTPUT_FORMATS.PLAIN) {
      const plain = {};

      if (scope.sentinel.version) {
        plain['Sentinel Version'] = scope.sentinel.version;
        plain['Sentinel Status'] = colors.sentinel(scope.sentinel.state)(scope.sentinel.state);
      }

      if (scope.syncAsset === MasternodeSyncAssetEnum.MASTERNODE_SYNC_FINISHED) {
        plain['Masternode State'] = (scope.state === MasternodeStateEnum.READY
          ? chalk.green : chalk.red)(scope.state);
      } else {
        plain['Masternode Sync Status'] = chalk.yellow(scope.syncAsset);
      }

      if (scope.state === MasternodeStateEnum.READY) {
        const {
          lastPaidHeight, lastPaidTime,
          paymentQueuePosition, nextPaymentTime,
          poSePenalty, enabledCount,
        } = scope.nodeState;

        plain['ProTx Hash'] = scope.proTxHash;
        plain['PoSe Penalty'] = colors.poSePenalty(poSePenalty, enabledCount)(`${poSePenalty}`);
        plain['Last paid block'] = lastPaidHeight;
        plain['Last paid time'] = lastPaidHeight === 0 ? 'Never' : lastPaidTime;
        plain['Payment queue position'] = paymentQueuePosition;
        plain['Next payment time'] = `in ${nextPaymentTime}`;
      }

      return printObject(plain, flags.format);
    }

    return printObject(scope, flags.format);
  }
}

MasternodeStatusCommand.description = 'Show masternode status details';

MasternodeStatusCommand.flags = {
  ...ConfigBaseCommand.flags,
  format: Flags.string({
    description: 'display output format',
    default: OUTPUT_FORMATS.PLAIN,
    options: Object.values(OUTPUT_FORMATS),
  }),
};

module.exports = MasternodeStatusCommand;
