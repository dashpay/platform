const chalk = require('chalk');

const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');
const CoreService = require('../../core/CoreService');
const blocksToTime = require('../../util/blocksToTime');
const getPaymentQueuePosition = require('../../util/getPaymentQueuePosition');
const getFormat = require('../../util/getFormat');
const printObject = require('../../printers/printObject');

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
    const coreService = new CoreService(
      config,
      createRpcClient(
        {
          port: config.get('core.rpc.port'),
          user: config.get('core.rpc.user'),
          pass: config.get('core.rpc.password'),
        },
      ),
      dockerCompose.docker.getContainer('core'),
    );

    if (config.get('core.masternode.enable') === false) {
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
        dmnState: masternodeDmnState,
        state: masternodeState,
        status: masternodeStatus,
        proTxHash: masternodeProTxHash,
      },
    } = await coreService.getRpcClient().masternode('status');

    let sentinelState = (await dockerCompose.execCommand(
      config.toEnvs(),
      'sentinel',
      'python bin/sentinel.py',
    )).out.split(/\r?\n/)[0];

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
    let lastPaidTime;
    if (masternodeState === 'READY') {
      paymentQueuePosition = getPaymentQueuePosition(
        masternodeDmnState, masternodeEnabledCount, coreBlocks,
      );

      // Determine last paid time
      if (masternodeDmnState.lastPaidHeight === 0) {
        lastPaidTime = 'Never';
      } else {
        lastPaidTime = `${blocksToTime(coreBlocks - masternodeDmnState.lastPaidHeight)} ago`;
      }
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
      if (masternodeDmnState.PoSePenalty === 0) {
        masternodePoSePenalty = chalk.green(masternodeDmnState.PoSePenalty);
      } else if (masternodeDmnState.PoSePenalty < masternodeEnabledCount) {
        masternodePoSePenalty = chalk.yellow(masternodeDmnState.PoSePenalty);
      } else {
        masternodePoSePenalty = chalk.red(masternodeDmnState.PoSePenalty);
      }
    }

    const outputRows = {
      'Masternode status': (masternodeState === 'READY' ? chalk.green : chalk.red)(masternodeStatus),
      'Sentinel status': (sentinelState !== '' ? sentinelState : 'No errors'),
    };

    if (masternodeState === 'READY') {
      outputRows['ProTx Hash'] = masternodeProTxHash;
      outputRows['PoSe Penalty'] = masternodePoSePenalty;
      outputRows['Last paid block'] = masternodeDmnState.lastPaidHeight;
      outputRows['Last paid time'] = lastPaidTime;
      outputRows['Payment queue position'] = `${paymentQueuePosition}/${masternodeEnabledCount}`;
      outputRows['Next payment time'] = `in ${blocksToTime(paymentQueuePosition)}`;
    }

    printObject(outputRows, getFormat(flags));
  }
}

MasternodeStatusCommand.description = 'Show masternode status details';

MasternodeStatusCommand.flags = {
  ...ConfigBaseCommand.flags,
};

module.exports = MasternodeStatusCommand;
