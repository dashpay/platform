const { table } = require('table');
const chalk = require('chalk');

const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');
const CoreService = require('../../core/CoreService');
const blocksToTime = require('../../util/blocksToTime');
const getPaymentQueuePosition = require('../../util/getPaymentQueuePosition');

const ContainerIsNotPresentError = require('../../docker/errors/ContainerIsNotPresentError');

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
  ) {
    const rows = [];

    const coreService = new CoreService(
      createRpcClient(
        {
          port: config.get('core.rpc.port'),
          user: config.get('core.rpc.user'),
          pass: config.get('core.rpc.password'),
        },
      ),
      dockerCompose.docker.getContainer('core'),
    );

    if (config.options.core.masternode.enable === false) {
      // eslint-disable-next-line no-console
      console.log('This is not a masternode!');
      this.exit();
    }

    // Collect data
    const { result: mnsyncStatus } = await coreService.getRpcClient().mnsync('status');
    const {
      result: {
        blocks: coreBlocks,
        verificationprogress: coreVerificationProgress,
      },
    } = await coreService.getRpcClient().getBlockchainInfo();

    const {
      result: {
        enabled: masternodeEnabledCount,
      },
    } = await coreService.getRpcClient().masternode('count');

    const {
      result: {
        dmnState: masternodeState,
        status: masternodeStatus,
        proTxHash: masternodeProTxHash,
      },
    } = await coreService.getRpcClient().masternode('status');

    let sentinelState = (await dockerCompose.execCommand(
      config.toEnvs(),
      'sentinel',
      'python bin/sentinel.py',
    )).out.split('\n')[0];

    // Determine status
    let status;
    try {
      ({
        State: {
          Status: status,
        },
      } = await dockerCompose.inspectService(config.toEnvs(), 'core'));
    } catch (e) {
      if (e instanceof ContainerIsNotPresentError) {
        status = 'not started';
      }
    }
    if (status === 'running' && mnsyncStatus.AssetName !== 'MASTERNODE_SYNC_FINISHED') {
      status = `syncing ${(coreVerificationProgress * 100).toFixed(2)}%`;
    }

    // Determine payment queue position
    let paymentQueuePosition;
    if (masternodeStatus === 'Ready' && masternodeState.PoSeRevivedHeight > 0) {
      paymentQueuePosition = getPaymentQueuePosition(
        masternodeState, masternodeEnabledCount, coreBlocks,
      );
    }

    // Apply colors
    if (status === 'running') {
      status = chalk.green(status);
    } else if (status.startsWith('syncing')) {
      status = chalk.yellow(status);
    } else {
      status = chalk.red(status);
    }

    if (sentinelState === '') {
      sentinelState = chalk.green('No errors');
    } else {
      sentinelState = chalk.red(sentinelState);
    }

    let masternodePoSePenalty;
    if (masternodeStatus === 'Ready') {
      if (masternodeState.PoSePenalty === 0) {
        masternodePoSePenalty = chalk.green(masternodeState.PoSePenalty);
      } else if (masternodeState.PoSePenalty < masternodeEnabledCount) {
        masternodePoSePenalty = chalk.yellow(masternodeState.PoSePenalty);
      } else {
        masternodePoSePenalty = chalk.red(masternodeState.PoSePenalty);
      }
    }

    // Build table
    rows.push(['Masternode status', (masternodeStatus === 'Ready' ? chalk.green : chalk.red)(masternodeStatus)]);
    rows.push(['Sentinel status', (sentinelState !== '' ? sentinelState : 'No errors')]);
    if (masternodeStatus === 'Ready') {
      rows.push(['ProTx Hash', masternodeProTxHash]);
      rows.push(['PoSe Penalty', masternodePoSePenalty]);
      rows.push(['Last paid block', masternodeState.lastPaidHeight]);
      rows.push(['Last paid time', `${blocksToTime(coreBlocks - masternodeState.lastPaidHeight)} ago`]);
      rows.push(['Payment queue position', `${paymentQueuePosition}/${masternodeEnabledCount}`]);
      rows.push(['Next payment time', `in ${blocksToTime(paymentQueuePosition)}`]);
    }

    const output = table(rows, { singleLine: true });

    // eslint-disable-next-line no-console
    console.log(output);
  }
}

MasternodeStatusCommand.description = 'Show masternode status details';

MasternodeStatusCommand.flags = {
  ...ConfigBaseCommand.flags,
};

module.exports = MasternodeStatusCommand;
