const fetch = require('node-fetch');
const chalk = require('chalk');

const ContainerIsNotPresentError = require('../docker/errors/ContainerIsNotPresentError');
const ServiceIsNotRunningError = require('../docker/errors/ServiceIsNotRunningError');

const CoreService = require('../core/CoreService');
const blocksToTime = require('../util/blocksToTime');
const getPaymentQueuePosition = require('../util/getPaymentQueuePosition');
const printObject = require('../printers/printObject');

/**
 *
 * @param {DockerCompose} dockerCompose
 * @param {createRpcClient} createRpcClient
 * @return {outputStatusOverview}
 */
function outputStatusOverviewFactory(
  dockerCompose,
  createRpcClient,
) {
  /**
   * @typedef {outputStatusOverview}
   * @param {Config} config
   * @param {string} format
   * @return void
   */
  async function outputStatusOverview(config, format) {
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

    if (!(await dockerCompose.isServiceRunning(config.toEnvs(), 'core'))) {
      throw new ServiceIsNotRunningError(config.get('network'), 'core');
    }

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
    if (config.get('core.masternode.enable')) {
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
    let platformStatus;

    if (config.get('network') !== 'mainnet' && config.name !== 'local_seed') {
      if (!(await dockerCompose.isServiceRunning(config.toEnvs(), 'drive_tenderdash'))) {
        try {
          ({
            State: {
              Status: platformStatus,
            },
          } = await dockerCompose.inspectService(config.toEnvs(), 'drive_tenderdash'));
        } catch (e) {
          if (e instanceof ContainerIsNotPresentError) {
            platformStatus = 'not started';
          }
        }
      } else if (coreIsSynced === true) {
        // Collecting platform data fails if Tenderdash is waiting for core to sync
        try {
          const platformStatusRes = await fetch(`http://localhost:${config.get('platform.drive.tenderdash.rpc.port')}/status`);
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
        } catch (e) {
          if (e.name === 'FetchError') {
            platformVersion = 'unknown';
            platformBlockHeight = 0;
            platformCatchingUp = false;
          } else {
            throw e;
          }
        }
      }
    }

    const platformExplorerURLs = {
      testnet: 'https://rpc.cloudwheels.net:26657',
      mainnet: '',
      local: '',
    };

    let explorerBlockHeight;
    if (platformExplorerURLs[config.get('network')] !== '') {
      try {
        const explorerBlockHeightRes = await fetch(`${platformExplorerURLs[config.get('network')]}/status`);
        ({
          result: {
            sync_info: {
              latest_block_height: explorerBlockHeight,
            },
          },
        } = await explorerBlockHeightRes.json());
      } catch (e) {
        if (e.name === 'FetchError') {
          explorerBlockHeight = 0;
        } else {
          throw e;
        }
      }
    } else {
      explorerBlockHeight = 0;
    }

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

    if (config.get('network') !== 'mainnet') {
      try {
        ({
          State: {
            Status: platformStatus,
          },
        } = await dockerCompose.inspectService(config.toEnvs(), 'drive_tenderdash'));
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
    if (config.get('core.masternode.enable') && masternodeStatus === 'Ready') {
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

    if (config.get('network') !== 'mainnet' && config.name !== 'local_seed') {
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

    const outputRows = {
      Network: coreChain,
      'Core Version': coreVersion.replace(/\/|\(.*?\)/g, ''),
      'Core Status': coreStatus,
    };

    if (config.get('core.masternode.enable')) {
      outputRows['Masternode Status'] = masternodeStatus;
    }

    if (config.get('network') !== 'mainnet' && config.name !== 'local_seed') {
      if (coreIsSynced === true
        && platformStatus !== chalk.red('not started')
        && platformStatus !== chalk.red('restarting')) {
        outputRows['Platform Version'] = platformVersion;
      }
      outputRows['Platform Status'] = platformStatus;
    }
    if (config.get('core.masternode.enable')) {
      if (masternodeStatus === 'Ready') {
        outputRows['PoSe Penalty'] = masternodeState.PoSePenalty;
        outputRows['Last paid block'] = masternodeState.lastPaidHeight;
        outputRows['Last paid time'] = `${blocksToTime(coreBlocks - masternodeState.lastPaidHeight)} ago`;
        outputRows['Payment queue position'] = `${paymentQueuePosition}/${masternodeEnabledCount}`;
        outputRows['Next payment time'] = `in ${blocksToTime(paymentQueuePosition)}`;
      }
    }

    printObject(outputRows, format);
  }

  return outputStatusOverview;
}

module.exports = outputStatusOverviewFactory;
