const { Flags } = require('@oclif/core');
const { OUTPUT_FORMATS } = require('../../constants');

const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');
const printObject = require('../../printers/printObject');

const ServiceIsNotRunningError = require('../../docker/errors/ServiceIsNotRunningError');

const colors = require('../../status/colors');

class PlatformStatusCommand extends ConfigBaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {DockerCompose} dockerCompose
   * @param {createRpcClient} createRpcClient
   * @param {Config} config
   * @param {getPlatformScope} getPlatformScope
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    flags,
    dockerCompose,
    createRpcClient,
    config,
    getPlatformScope,
  ) {
    if (config.get('network') === 'mainnet') {
      // eslint-disable-next-line no-console
      console.log('Platform is not supported on mainnet yet!');
      this.exit();
    }

    if (!(await dockerCompose.isServiceRunning(config.toEnvs(), 'drive_tenderdash'))) {
      throw new ServiceIsNotRunningError(config.get('network'), 'drive_tenderdash');
    }

    // Collect platform data
    const scope = await getPlatformScope();

    // Collecting platform data fails if Tenderdash is waiting for core to sync
    if (!scope.coreIsSynced) {
      // eslint-disable-next-line no-console
      console.log('Platform status is not available until core sync is complete!');
      this.exit();
    }

    const {
      status,
      httpService,
      httpPort,
      httpPortState,
      gRPCService,
      gRPCPort,
      gRPCPortState,
      p2pService,
      p2pPort,
      p2pPortState,
      rpcService,
      tenderdash,
    } = scope;

    const plain = {
      Status: status,
      'HTTP service': httpService,
      'HTTP port': `${httpPort} ${colors.portState(httpPortState)(httpPortState)}`,
      'GRPC service': gRPCService,
      'GRPC port': `${gRPCPort} ${colors.portState(gRPCPortState)(gRPCPortState)}`,
      'P2P service': p2pService,
      'P2P port': `${p2pPort} ${colors.portState(p2pPortState)(p2pPortState)}`,
      'RPC service': rpcService,
    };

    if (tenderdash.version) {
      const {
        version: tenderdashVersion,
        lastBlockHeight: platformBlockHeight,
        latestAppHash: platformLatestAppHash,
        peers: platformPeers,
        network: tenderdashNetwork,
      } = tenderdash;

      plain['Tenderdash Version'] = tenderdashVersion;
      plain['Block height'] = platformBlockHeight;
      plain['Peer count'] = platformPeers;
      plain['App hash'] = platformLatestAppHash;

      plain.Network = tenderdashNetwork;
    }

    return printObject(plain, flags.format);
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
