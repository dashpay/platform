const chalk = require('chalk');

const { Flags } = require('@oclif/core');
const { OUTPUT_FORMATS } = require('../../constants');

const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');
const printObject = require('../../printers/printObject');
const colors = require('../../status/colors');
const MasternodeStateEnum = require('../../status/enums/masternodeState');
const MasternodeSyncAssetEnum = require('../../status/enums/masternodeSyncAsset');

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
    const plain = {
      'Masternode State': 'n/a',
      'Masternode Sync Status': 'n/a',
      'ProTx Hash': 'n/a',
      'PoSe Penalty': 'n/a',
      'Last paid block': 'n/a',
      'Last paid time': 'n/a',
      'Enabled count': 'n/a',
      'Payment queue position': 'n/a',
      'Next payment time': 'n/a',
    };

    if (config.get('core.masternode.enable') === false) {
      if (process.env.DEBUG) {
        // eslint-disable-next-line no-console
        console.error('This is not a masternode!');
      }
    }

    const scope = await getMasternodeScope(config);

    if (flags.format === OUTPUT_FORMATS.PLAIN) {
      if (scope.syncAsset === MasternodeSyncAssetEnum.MASTERNODE_SYNC_FINISHED) {
        plain['Masternode State'] = (scope.state === MasternodeStateEnum.READY
          ? chalk.green : chalk.red)(scope.state) || 'n/a';
      } else {
        plain['Masternode Sync Status'] = scope.syncAsset ? chalk.yellow(scope.syncAsset) : 'n/a';
      }

      plain['Total Masternodes'] = scope.masternodeTotal ?? 'n/a';
      plain['Enabled Masternodes'] = scope.masternodeEnabled ?? 'n/a';
      plain['Total Evonodes'] = scope.evonodeTotal ?? 'n/a';
      plain['Enabled Evonodes'] = scope.evonodeEnabled ?? 'n/a';

      if (scope.state === MasternodeStateEnum.READY) {
        const {
          lastPaidHeight, lastPaidTime,
          paymentQueuePosition, nextPaymentTime,
          poSePenalty,
        } = scope.nodeState;

        plain['ProTx Hash'] = scope.proTxHash || 'n/a';
        plain['PoSe Penalty'] = colors.poSePenalty(poSePenalty,
          scope.masternodeEnabled, scope.evonodeEnabled)(`${poSePenalty}`) || 'n/a';
        plain['Last paid block'] = lastPaidHeight ?? 'n/a';
        plain['Last paid time'] = lastPaidHeight === 0 ? 'Never' : (lastPaidTime || 'n/a');
        plain['Payment queue position'] = paymentQueuePosition ?? 'n/a';
        plain['Next payment time'] = `in ${nextPaymentTime}` || 'n/a';
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
