const { table } = require('table');
const fetch = require('node-fetch');
const chalk = require('chalk');

const BaseCommand = require('../../oclif/command/BaseCommand');
const CoreService = require('../../core/CoreService');

const ContainerIsNotPresentError = require('../../docker/errors/ContainerIsNotPresentError');

class CoreStatusCommand extends BaseCommand {
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
    if (config.options.network === 'mainnet') {
      // eslint-disable-next-line no-console
      console.log('Platform is not supported on mainnet yet!');
      this.exit();
    }

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

    const explorerURLs = {
      evonet: 'https://rpc.cloudwheels.net:26657/status',
    };

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
    const tendermintStatusRes = await fetch(`http://localhost:${config.options.platform.drive.tendermint.rpc.port}/status`);
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
    } = await tendermintStatusRes.json();

    const tendermintNetInfoRes = await fetch(`http://localhost:${config.options.platform.drive.tendermint.rpc.port}/net_info`);
    const {
      result: {
        n_peers: platformPeers,
      },
    } = await tendermintNetInfoRes.json();

    let explorerLatestBlockHeight;
    if (explorerURLs[config.options.network]) {
      const explorerBlockHeightRes = await fetch(explorerURLs[config.options.network]);
      ({
        result: {
          sync_info: {
            latest_block_height: explorerLatestBlockHeight,
          },
        },
      } = await explorerBlockHeightRes.json());
    }

    // Check ports
    const httpPortStateRes = await fetch(`https://mnowatch.org/${config.options.platform.dapi.nginx.http.port}/`);
    let httpPortState = await httpPortStateRes.text();
    const gRpcPortStateRes = await fetch(`https://mnowatch.org/${config.options.platform.dapi.nginx.grpc.port}/`);
    let gRpcPortState = await gRpcPortStateRes.text();
    const p2pPortStateRes = await fetch(`https://mnowatch.org/${config.options.platform.drive.tendermint.p2p.port}/`);
    let p2pPortState = await p2pPortStateRes.text();

    // Determine status
    let status;
    try {
      ({
        State: {
          Status: status,
        },
      } = await dockerCompose.inspectService(config.toEnvs(), 'drive_tendermint'));
    } catch (e) {
      if (e instanceof ContainerIsNotPresentError) {
        status = 'not started';
      }
    }
    if (status === 'running' && platformCatchingUp === true && explorerURLs[config.options.network]) {
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
    if (explorerURLs[config.options.network]) {
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

    // Build table
    rows.push(['Tenderdash Version', platformVersion]);
    rows.push(['Network', platformNetwork]);
    rows.push(['Status', status]);
    rows.push(['Block height', blocks]);
    if (explorerURLs[config.options.network]) {
      rows.push(['Remote block height', explorerLatestBlockHeight]);
    }
    rows.push(['Peer count', platformPeers]);
    rows.push(['App hash', platformLatestAppHash]);
    rows.push(['HTTP service', `${config.options.externalIp}:${config.options.platform.dapi.nginx.http.port}`]);
    rows.push(['HTTP port', `${config.options.platform.dapi.nginx.http.port} ${httpPortState}`]);
    rows.push(['gRPC service', `${config.options.externalIp}:${config.options.platform.dapi.nginx.grpc.port}`]);
    rows.push(['gRPC port', `${config.options.platform.dapi.nginx.grpc.port} ${gRpcPortState}`]);
    rows.push(['P2P service', `${config.options.externalIp}:${config.options.platform.drive.tendermint.p2p.port}`]);
    rows.push(['P2P port', `${config.options.platform.drive.tendermint.p2p.port} ${p2pPortState}`]);
    rows.push(['RPC service', `127.0.0.1:${config.options.platform.drive.tendermint.rpc.port}`]);
    const output = table(rows, { singleLine: true });

    // eslint-disable-next-line no-console
    console.log(output);
  }
}

CoreStatusCommand.description = 'Show platform status details';

CoreStatusCommand.flags = {
  ...BaseCommand.flags,
};

module.exports = CoreStatusCommand;
