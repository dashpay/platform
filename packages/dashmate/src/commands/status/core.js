const fetch = require('node-fetch');
const chalk = require('chalk');

const { Flags } = require('@oclif/core');
const { OUTPUT_FORMATS } = require('../../constants');

const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');
const CoreService = require('../../core/CoreService');
const printObject = require('../../printers/printObject');

const ContainerIsNotPresentError = require('../../docker/errors/ContainerIsNotPresentError');

class CoreStatusCommand extends ConfigBaseCommand {
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

    const insightURLs = {
      testnet: 'https://testnet-insight.dashevo.org/insight-api',
      mainnet: 'https://insight.dash.org/insight-api',
    };

    // Collect data
    const {
      result: {
        blocks: coreBlocks,
        chain: coreChain,
        difficulty: coreDifficulty,
        headers: coreHeaders,
        verificationprogress: coreVerificationProgress,
      },
    } = await coreService.getRpcClient().getBlockchainInfo();
    const { result: networkInfo } = await coreService.getRpcClient().getNetworkInfo();
    const { result: mnsyncStatus } = await coreService.getRpcClient().mnsync('status');
    const { result: peerInfo } = await coreService.getRpcClient().getPeerInfo();

    let latestVersion;
    try {
      const latestVersionRes = await fetch('https://api.github.com/repos/dashpay/dash/releases/latest');
      latestVersion = (await latestVersionRes.json()).tag_name.substring(1);
    } catch (e) {
      if (e.name === 'FetchError') {
        latestVersion = '0';
      } else {
        throw e;
      }
    }

    let corePortState;
    try {
      const corePortStateRes = await fetch(`https://mnowatch.org/${config.get('core.p2p.port')}/`);
      corePortState = await corePortStateRes.text();
    } catch (e) {
      if (e.name === 'FetchError') {
        corePortState = 'ERROR';
      } else {
        throw e;
      }
    }

    let coreVersion = networkInfo.subversion.replace(/\/|\(.*?\)|Dash Core:/g, '');
    let explorerBlockHeight;
    if (insightURLs[config.get('network')]) {
      try {
        const explorerBlockHeightRes = await fetch(`${insightURLs[config.get('network')]}/status`);
        ({
          info: {
            blocks: explorerBlockHeight,
          },
        } = await explorerBlockHeightRes.json());
      } catch (e) {
        if (e.name === 'FetchError') {
          explorerBlockHeight = 0;
        } else {
          throw e;
        }
      }
    }

    let sentinelVersion;
    let sentinelState;
    if (config.get('core.masternode.enable')) {
      sentinelVersion = (await dockerCompose.execCommand(
        config.toEnvs(),
        'sentinel',
        'python bin/sentinel.py -v',
      )).out.split(/\r?\n/)[0].replace(/Dash Sentinel v/, '');
      // eslint-disable-next-line prefer-destructuring
      sentinelState = (await dockerCompose.execCommand(
        config.toEnvs(),
        'sentinel',
        'python bin/sentinel.py',
      )).out.split(/\r?\n/)[0];
    }

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

    // Apply colors
    if (status === 'running') {
      status = chalk.green(status);
    } else if (status.startsWith('syncing')) {
      status = chalk.yellow(status);
    } else {
      status = chalk.red(status);
    }

    if (coreVersion === latestVersion) {
      coreVersion = chalk.green(coreVersion);
    } else if (coreVersion.match(/\d+.\d+/)[0] === latestVersion.match(/\d+.\d+/)[0]) {
      coreVersion = chalk.yellow(coreVersion);
    } else {
      coreVersion = chalk.red(coreVersion);
    }

    if (corePortState === 'OPEN') {
      corePortState = chalk.green(corePortState);
    } else {
      corePortState = chalk.red(corePortState);
    }

    let blocks;
    if (coreBlocks === coreHeaders || coreBlocks >= explorerBlockHeight) {
      blocks = chalk.green(coreBlocks);
    } else if ((explorerBlockHeight - coreBlocks) < 3) {
      blocks = chalk.yellow(coreBlocks);
    } else {
      blocks = chalk.red(coreBlocks);
    }

    if (config.get('core.masternode.enable')) {
      if (sentinelState === '') {
        sentinelState = chalk.green('No errors');
      } else {
        sentinelState = chalk.red(sentinelState);
      }
    }

    const outputRows = {
      Version: coreVersion,
      'Latest version': latestVersion,
      Network: coreChain,
      Status: status,
      'Sync asset': mnsyncStatus.AssetName,
      'Peer count': peerInfo.length,
      'P2P service': `${config.get('externalIp')}:${config.get('core.p2p.port')}`,
      'P2P port': `${config.get('core.p2p.port')} ${corePortState}`,
      'RPC service': `127.0.0.1:${config.get('core.rpc.port')}`,
      'Block height': blocks,
      'Header height': coreHeaders,
      Difficulty: coreDifficulty,
    };

    if (config.get('core.masternode.enable')) {
      outputRows['Sentinel version'] = sentinelVersion;
      outputRows['Sentinel status'] = (sentinelState);
    }

    if (insightURLs[config.get('network')]) {
      outputRows['Remote block height'] = explorerBlockHeight;
    }

    printObject(outputRows, flags.format);
  }
}

CoreStatusCommand.description = 'Show core status details';

CoreStatusCommand.flags = {
  ...ConfigBaseCommand.flags,
  format: Flags.string({
    description: 'display output format',
    default: OUTPUT_FORMATS.PLAIN,
    options: Object.values(OUTPUT_FORMATS),
  }),
};

module.exports = CoreStatusCommand;
