const { table } = require('table');
const fetch = require('node-fetch');
const chalk = require('chalk');

const ContainerIsNotPresentError = require('../../docker/errors/ContainerIsNotPresentError');

const BaseCommand = require('../../oclif/command/BaseCommand');
const CoreService = require('../../core/CoreService');
const blocksToTime = require('../../util/blocksToTime');
const getPaymentQueuePosition = require('../../util/getPaymentQueuePosition');

class StatusCommand extends BaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {DockerCompose} dockerCompose
   * @param {CoreService} coreService
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

    // Collect core data
    const {
      result: {
        AssetName: coreSyncAsset,
        IsSynced: coreIsSynced,
      },
    } = await coreService.getRpcClient().mnsync('status');

    let {
      result: {
        subversion: coreVersion,
      },
    } = await coreService.getRpcClient().getNetworkInfo();
    coreVersion = coreVersion.replace(/\/|\(.*?\)|Dash Core:/g, '');

    const {
      result: {
        blocks: coreBlocks,
        chain: coreChain,
        verificationprogress: coreVerificationProgress,
      },
    } = await coreService.getRpcClient().getBlockchainInfo();

    // Collect masternode data
    let masternodeState;
    let masternodeStatus;
    let masternodeEnabledCount;
    if (config.options.core.masternode.enable === true) {
      ({
        result: {
          dmnState: masternodeState,
          status: masternodeStatus,
        },
      } = await coreService.getRpcClient().masternode('status'));
      ({
        result: {
          enabled: masternodeEnabledCount,
        },
      } = await coreService.getRpcClient().masternode('count'));
    }

    // Collect platform data
    let platformVersion;
    let platformBlockHeight;
    let platformCatchingUp;
    // Collecting platform data fails if Tenderdash is waiting for core to sync
    if (config.options.network !== 'testnet' && coreIsSynced === true) {
      const platformStatusRes = await fetch(`http://localhost:${config.options.platform.drive.tendermint.rpc.port}/status`);
      ({
        result: {
          node_info: {
            version: platformVersion,
          },
          sync_info: {
            latest_block_height: platformBlockHeight,
            catching_up: platformCatchingUp,
          },
        },
      } = await platformStatusRes.json());
    }

    const explorerBlockHeightRes = await fetch('https://rpc.cloudwheels.net:26657/status');
    const {
      result: {
        sync_info: {
          latest_block_height: explorerBlockHeight,
        },
      },
    } = await explorerBlockHeightRes.json();

    // Determine status
    let coreStatus;
    try {
      ({
        State: {
          Status: coreStatus,
        },
      } = await dockerCompose.inspectService(config.toEnvs(), 'core'));
    } catch (e) {
      if (e instanceof ContainerIsNotPresentError) {
        coreStatus = 'not started';
      }
    }
    if (coreStatus === 'running' && coreSyncAsset !== 'MASTERNODE_SYNC_FINISHED') {
      coreStatus = `syncing ${(coreVerificationProgress * 100).toFixed(2)}%`;
    }

    let platformStatus;
    if (config.options.network !== 'testnet') {
      try {
        ({
          State: {
            Status: platformStatus,
          },
        } = await dockerCompose.inspectService(config.toEnvs(), 'drive_tendermint'));
      } catch (e) {
        if (e instanceof ContainerIsNotPresentError) {
          platformStatus = 'not started';
        }
      }
      if (platformStatus === 'running' && coreIsSynced === false) {
        platformStatus = 'waiting for core sync';
      } else if (platformStatus === 'running' && platformCatchingUp === true) {
        platformStatus = `syncing ${((platformBlockHeight / explorerBlockHeight) * 100).toFixed(2)}%`;
      }
    }

    // Determine payment queue position
    let paymentQueuePosition;
    if (config.options.core.masternode.enable === true && masternodeStatus === 'Ready') {
      paymentQueuePosition = getPaymentQueuePosition(
        masternodeState, masternodeEnabledCount, coreBlocks,
      );
    }

    // Apply colors
    if (coreStatus === 'running') {
      coreStatus = chalk.green(coreStatus);
    } else if (coreStatus.includes('syncing')) {
      coreStatus = chalk.yellow(coreStatus);
    } else {
      coreStatus = chalk.red(coreStatus);
    }

    if (config.options.network !== 'testnet') {
      if (platformStatus === 'running') {
        platformStatus = chalk.green(platformStatus);
      } else if (platformStatus.startsWith('syncing')) {
        platformStatus = chalk.yellow(platformStatus);
      } else {
        platformStatus = chalk.red(platformStatus);
      }
    }

    if (masternodeStatus === 'Ready') {
      masternodeStatus = chalk.green(masternodeStatus);
    } else {
      masternodeStatus = chalk.red(masternodeStatus);
    }

    // Build table
    rows.push(['Network', coreChain]);
    rows.push(['Core Version', coreVersion.replace(/\/|\(.*?\)/g, '')]);
    rows.push(['Core Status', coreStatus]);
    if (config.options.core.masternode.enable === true) {
      rows.push(['Masternode Status', (masternodeStatus)]);
    }
    if (config.options.network !== 'testnet') {
      if (coreIsSynced === true) {
        rows.push(['Platform Version', platformVersion]);
      }
      rows.push(['Platform Status', platformStatus]);
    }
    if (config.options.core.masternode.enable === true) {
      if (masternodeStatus === 'Ready') {
        rows.push(['PoSe Penalty', masternodeState.PoSePenalty]);
        rows.push(['Last paid block', masternodeState.lastPaidHeight]);
        rows.push(['Last paid time', `${blocksToTime(coreBlocks - masternodeState.lastPaidHeight)} ago`]);
        rows.push(['Payment queue position', `${paymentQueuePosition}/${masternodeEnabledCount}`]);
        rows.push(['Next payment time', `in ${blocksToTime(paymentQueuePosition)}`]);
      }
    }

    const output = table(rows, { singleLine: true });

    // eslint-disable-next-line no-console
    console.log(output);
  }
}

StatusCommand.description = 'Show status overview';

StatusCommand.flags = {
  ...BaseCommand.flags,
};

module.exports = StatusCommand;
