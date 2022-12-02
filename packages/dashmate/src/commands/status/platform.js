const chalk = require('chalk');

const { Flags } = require('@oclif/core');
const { OUTPUT_FORMATS } = require('../../constants');

const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');
const CoreService = require('../../core/CoreService');
const printObject = require('../../printers/printObject');

const ServiceIsNotRunningError = require('../../docker/errors/ServiceIsNotRunningError');

const providers = require('../../status/providers')
const colors = require("../../status/colors");

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
    outputStatusOverview,
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

    if (!(await dockerCompose.isServiceRunning(config.toEnvs(), 'drive_tenderdash'))) {
      throw new ServiceIsNotRunningError(config.get('network'), 'drive_tenderdash');
    }

    const {core} = await outputStatusOverview(config, ['core'])

    // Collecting platform data fails if Tenderdash is waiting for core to sync
    if (core.isSynced === false) {
      // eslint-disable-next-line no-console
      console.log('Platform status is not available until core sync is complete!');
      this.exit();
    }

    // Collect platform data
    const {platform} = await outputStatusOverview(config, ['platform'])
    const {
      status: platformStatus,
      tenderdash
    } = platform
    const {
      version: tenderdashVersion,
      lastBlockHeight: platformBlockHeight,
      latestAppHash: platformLatestAppHash,
      peers: platformPeers,
      network: tenderdashNetwork
    } = tenderdash

    // Check ports
    const httpState = await providers.mnowatch.checkPortStatus(config.get('platform.dapi.envoy.http.port'));
    const gRpcState = await providers.mnowatch.checkPortStatus(config.get('platform.dapi.envoy.grpc.port'));
    const p2pState = await providers.mnowatch.checkPortStatus(config.get('platform.drive.tenderdash.p2p.port'));

    const json  = {
      tenderdashVersion,
      network: tenderdashNetwork,
      status: platformStatus,
      blockHeight: platformBlockHeight,
      peerCount: platformPeers,
      appHash: platformLatestAppHash,
      httpService: `${config.get('externalIp')}:${config.get('platform.dapi.envoy.http.port')}`,
      httpPort: httpState,
      gRPCService: `${config.get('externalIp')}:${config.get('platform.dapi.envoy.grpc.port')}`,
      gRPCPort: gRpcState,
      p2pService: `${config.get('externalIp')}:${config.get('platform.drive.tenderdash.p2p.port')}`,
      p2pPortState: p2pState,
      rpcService: `127.0.0.1:${config.get('platform.drive.tenderdash.rpc.port')}`,
    };

    let status
    const outputRows = {
      'Tenderdash Version': tenderdashVersion,
      'Network': tenderdashNetwork,
      'Status': status,
      'Block height': platformBlockHeight,
      'Peer count': platformPeers,
      'App hash': platformLatestAppHash,
      'HTTP service': `${config.get('externalIp')}:${config.get('platform.dapi.envoy.http.port')}`,
      'HTTP port': `${config.get('platform.dapi.envoy.http.port')} ${colors.portState(p2pState)(httpState)}`,
      'gRPC service': `${config.get('externalIp')}:${config.get('platform.dapi.envoy.grpc.port')}`,
      'gRPC port': `${config.get('platform.dapi.envoy.grpc.port')} ${colors.portState(p2pState)(gRpcState)}`,
      'P2P service': `${config.get('externalIp')}:${config.get('platform.drive.tenderdash.p2p.port')}`,
      'P2P port': `${config.get('platform.drive.tenderdash.p2p.port')} ${colors.portState(p2pState)(p2pState)}`,
      'RPC service': `127.0.0.1:${config.get('platform.drive.tenderdash.rpc.port')}`,
    };

    printObject(outputRows, flags.format);
  }
}

PlatformStatusCommand.description = 'Show platform status details';

PlatformStatusCommand.flags = {
  ...ConfigBaseCommand.flags,
  format: Flags.string({
    description: 'display output format',
    default: OUTPUT_FORMATS.PLAIN,
    options: Object.values(OUTPUT_FORMATS),
  }),
};

module.exports = PlatformStatusCommand;
