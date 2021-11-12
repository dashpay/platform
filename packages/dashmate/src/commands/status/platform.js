const fetch = require('node-fetch');
const chalk = require('chalk');

const { flags: flagTypes } = require('@oclif/command');
const { OUTPUT_FORMATS } = require('../../constants');

const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');
const CoreService = require('../../core/CoreService');
const printObject = require('../../printers/printObject');

const ContainerIsNotPresentError = require('../../docker/errors/ContainerIsNotPresentError');
const ServiceIsNotRunningError = require('../../docker/errors/ServiceIsNotRunningError');

class PlatformStatusCommand extends ConfigBaseCommand {
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
    if (config.get('network') === 'mainnet') {
      // eslint-disable-next-line no-console
      console.log('Platform is not supported on mainnet yet!');
      this.exit();
    }

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

    const explorerURLs = {
      testnet: 'https://rpc.cloudwheels.net:26657',
      mainnet: '',
    };

    if (!(await dockerCompose.isServiceRunning(config.toEnvs(), 'drive_tenderdash'))) {
      throw new ServiceIsNotRunningError(config.get('network'), 'drive_tenderdash');
    }

    // Collect core data
    const {
      result: {
        IsSynced: coreIsSynced,
      },
    } = await coreService.getRpcClient().mnsync('status');

    // Collecting platform data fails if Tenderdash is waiting for core to sync
    if (coreIsSynced === false) {
      // eslint-disable-next-line no-console
      console.log('Platform status is not available until core sync is complete!');
      this.exit();
    }

    // Collect platform data
    const tenderdashStatusRes = await fetch(`http://localhost:${config.get('platform.drive.tenderdash.rpc.port')}/status`);
    const {
      result: {
        node_info: {
          version: platformVersion,
          network: platformNetwork,
        },
        sync_info: {
          catching_up: platformCatchingUp,
          latest_app_hash: platformLatestAppHash,
          latest_block_height: platformLatestBlockHeight,
        },
      },
    } = await tenderdashStatusRes.json();

    const tenderdashNetInfoRes = await fetch(`http://localhost:${config.get('platform.drive.tenderdash.rpc.port')}/net_info`);
    const {
      result: {
        n_peers: platformPeers,
      },
    } = await tenderdashNetInfoRes.json();

    let explorerLatestBlockHeight;
    if (explorerURLs[config.get('network')]) {
      try {
        const explorerBlockHeightRes = await fetch(`${explorerURLs[config.get('network')]}/status`);
        ({
          result: {
            sync_info: {
              latest_block_height: explorerLatestBlockHeight,
            },
          },
        } = await explorerBlockHeightRes.json());
      } catch (e) {
        if (e.name === 'FetchError') {
          explorerLatestBlockHeight = 0;
        } else {
          throw e;
        }
      }
    }

    // Check ports
    let httpPortState;
    let gRpcPortState;
    let p2pPortState;
    try {
      const httpPortStateRes = await fetch(`https://mnowatch.org/${config.get('platform.dapi.envoy.http.port')}/`);
      httpPortState = await httpPortStateRes.text();
      const gRpcPortStateRes = await fetch(`https://mnowatch.org/${config.get('platform.dapi.envoy.grpc.port')}/`);
      gRpcPortState = await gRpcPortStateRes.text();
      const p2pPortStateRes = await fetch(`https://mnowatch.org/${config.get('platform.drive.tenderdash.p2p.port')}/`);
      p2pPortState = await p2pPortStateRes.text();
    } catch (e) {
      if (e.name === 'FetchError') {
        httpPortState = 'ERROR';
        gRpcPortState = 'ERROR';
        p2pPortState = 'ERROR';
      } else {
        throw e;
      }
    }

    // Determine status
    let status;
    try {
      ({
        State: {
          Status: status,
        },
      } = await dockerCompose.inspectService(config.toEnvs(), 'drive_tenderdash'));
    } catch (e) {
      if (e instanceof ContainerIsNotPresentError) {
        status = 'not started';
      }
    }
    if (status === 'running' && platformCatchingUp === true && explorerURLs[config.get('network')]) {
      status = `syncing ${((platformLatestBlockHeight / explorerLatestBlockHeight) * 100).toFixed(2)}%`;
    }

    // Apply colors
    if (status === 'running') {
      status = chalk.green(status);
    } else if (status.includes('syncing')) {
      status = chalk.yellow(status);
    } else {
      status = chalk.red(status);
    }

    let blocks;
    if (explorerURLs[config.get('network')]) {
      if (platformLatestBlockHeight >= explorerLatestBlockHeight) {
        blocks = chalk.green(platformLatestBlockHeight);
      } else {
        blocks = chalk.red(platformLatestBlockHeight);
      }
    } else {
      blocks = platformLatestBlockHeight;
    }

    if (httpPortState === 'OPEN') {
      httpPortState = chalk.green(httpPortState);
    } else {
      httpPortState = chalk.red(httpPortState);
    }
    if (gRpcPortState === 'OPEN') {
      gRpcPortState = chalk.green(gRpcPortState);
    } else {
      gRpcPortState = chalk.red(gRpcPortState);
    }
    if (p2pPortState === 'OPEN') {
      p2pPortState = chalk.green(p2pPortState);
    } else {
      p2pPortState = chalk.red(p2pPortState);
    }

    const outputRows = {
      'Tenderdash Version': platformVersion,
      Network: platformNetwork,
      Status: status,
      'Block height': blocks,
      'Peer count': platformPeers,
      'App hash': platformLatestAppHash,
      'HTTP service': `${config.get('externalIp')}:${config.get('platform.dapi.envoy.http.port')}`,
      'HTTP port': `${config.get('platform.dapi.envoy.http.port')} ${httpPortState}`,
      'gRPC service': `${config.get('externalIp')}:${config.get('platform.dapi.envoy.grpc.port')}`,
      'gRPC port': `${config.get('platform.dapi.envoy.grpc.port')} ${gRpcPortState}`,
      'P2P service': `${config.get('externalIp')}:${config.get('platform.drive.tenderdash.p2p.port')}`,
      'P2P port': `${config.get('platform.drive.tenderdash.p2p.port')} ${p2pPortState}`,
      'RPC service': `127.0.0.1:${config.get('platform.drive.tenderdash.rpc.port')}`,
    };

    if (explorerURLs[config.get('network')]) {
      outputRows['Remote block height'] = explorerLatestBlockHeight;
    }

    printObject(outputRows, flags.format);
  }
}

PlatformStatusCommand.description = 'Show platform status details';

PlatformStatusCommand.flags = {
  ...ConfigBaseCommand.flags,
  format: flagTypes.string({
    description: 'display output format',
    default: OUTPUT_FORMATS.PLAIN,
    options: Object.values(OUTPUT_FORMATS),
  }),
};

module.exports = PlatformStatusCommand;
