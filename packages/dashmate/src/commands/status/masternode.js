const chalk = require('chalk');

const { Flags } = require('@oclif/core');
const { OUTPUT_FORMATS } = require('../../constants');

const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');
const printObject = require('../../printers/printObject');
const colors = require('../../status/colors');
const MasternodeStateEnum = require('../../enums/masternodeState');

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
    statusProvider,
  ) {
    if (config.get('core.masternode.enable') === false) {
      // eslint-disable-next-line no-console
      console.log('This is not a masternode!');
      this.exit();
    }

    const scope = await statusProvider.getMasternodeScope();

    const { core, masternode } = scope;
    const { verificationProgress } = core;

    if (flags.format === OUTPUT_FORMATS.PLAIN) {
      const plain = {};

      plain['Masternode Status'] = colors.status(masternode.status)(masternode.status);
      plain['Masternode State'] = (masternode.state === 'READY' ? chalk.green : chalk.red)(masternode.state);
      plain['Verification Progress'] = `${verificationProgress * 100}%`;
      plain['Sentinel Status'] = (masternode.sentinelState !== '' ? chalk.red(masternode.sentinelState) : chalk.green('No errors'));

      if (masternode.state === MasternodeStateEnum.READY) {
        const {
          proTxHash, lastPaidBlock, lastPaidTime,
          paymentQueuePosition, nexPaymentTime,
          poSePenalty, enabledCount,
        } = masternode;

        plain['ProTx Hash'] = proTxHash;
        plain['PoSe Penalty'] = colors.poSePenalty(poSePenalty, enabledCount)(poSePenalty);
        plain['Last paid block'] = lastPaidBlock;
        plain['Last paid time'] = lastPaidBlock === 0 ? 'Never' : lastPaidTime;
        plain['Payment queue position'] = paymentQueuePosition;
        plain['Next payment time'] = `in ${nexPaymentTime}`;
      }

      return printObject(plain, flags.format);
    }

    printObject(scope, flags.format);
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
