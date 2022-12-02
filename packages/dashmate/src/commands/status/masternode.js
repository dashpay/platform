const chalk = require('chalk');

const {Flags} = require('@oclif/core');
const {OUTPUT_FORMATS} = require('../../constants');

const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');
const printObject = require('../../printers/printObject');

class MasternodeStatusCommand extends ConfigBaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {DockerCompose} dockerCompose
   * @param {createRpcClient} createRpcClient
   * @param {Config} config
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    flags,
    dockerCompose,
    createRpcClient,
    config,
    outputStatusOverview
  ) {
    if (config.get('core.masternode.enable') === false) {
      // eslint-disable-next-line no-console
      console.log('This is not a masternode!');
      this.exit();
    }

    const status = await outputStatusOverview(config, ['core', 'masternode'])

    const {core, masternode} = status
    const {verificationProgress} = core

    let toPrint = masternode

    if (flags.format === OUTPUT_FORMATS.PLAIN) {
      let masternodeStatus = masternode.status === 'syncing' ?
        `syncing ${(verificationProgress * 100).toFixed(2)}%` : masternode.status

      if (masternodeStatus === 'running') {
        masternodeStatus = chalk.green(masternodeStatus);
      } else if (status.startsWith('syncing')) {
        masternodeStatus = chalk.yellow(masternodeStatus);
      } else {
        masternodeStatus = chalk.red(masternodeStatus);
      }

      toPrint = {
        'Masternode status': (masternode.state === 'READY' ? chalk.green : chalk.red)(masternodeStatus),
        'Sentinel status': (masternode.sentinelState !== '' ? chalk.red(masternode.sentinelState) : chalk.green('No errors')),
      };

      if (masternode.state === 'READY') {
        const {
          proTxHash, lastPaidBlock, lastPaidTime,
          paymentQueuePosition, nexPaymentTime
        } = masternode

        let {poSePenalty, enabledCount} = masternode

        if (poSePenalty === 0) {
          poSePenalty = chalk.green(poSePenalty);
        } else if (poSePenalty < enabledCount) {
          poSePenalty = chalk.yellow(poSePenalty);
        } else {
          poSePenalty = chalk.red(poSePenalty);
        }

        toPrint['ProTx Hash'] = proTxHash;
        toPrint['PoSe Penalty'] = poSePenalty;
        toPrint['Last paid block'] = lastPaidBlock;
        toPrint['Last paid time'] = lastPaidBlock === 0 ? 'Never' : lastPaidTime;
        toPrint['Payment queue position'] = paymentQueuePosition;
        toPrint['Next payment time'] = `in ${nexPaymentTime}`;
      }
    }

    printObject(toPrint, flags.format);
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
