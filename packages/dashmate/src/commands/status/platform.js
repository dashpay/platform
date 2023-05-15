/* eslint-disable dot-notation */
const { Flags } = require('@oclif/core');
const { OUTPUT_FORMATS } = require('../../constants');

const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');
const printObject = require('../../printers/printObject');

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
    const plain = {
      'HTTP service': 'n/a',
      'HTTP port': 'n/a',
      'P2P service': 'n/a',
      'P2P port': 'n/a',
      'RPC service': 'n/a',
      'Tenderdash Docker Status': 'n/a',
      'Tenderdash Service Status': 'n/a',
      'Drive Docker Status': 'n/a',
      'Drive Service Status': 'n/a',
      Network: 'n/a',
      'Tenderdash Version': 'n/a',
      'Block height': 'n/a',
      'Peer count': 'n/a',
      'App hash': 'n/a',
    };

    if (!config.get('platform.enable')) {
      if (process.env.DEBUG) {
        // eslint-disable-next-line no-console
        console.error('Platform is not supported for this node type and network');
      }
    }

    if (!(await dockerCompose.isServiceRunning(config.toEnvs(), 'drive_tenderdash'))) {
      if (process.env.DEBUG) {
        // eslint-disable-next-line no-console
        console.error('Platform is not running');
      }
    }

    // Collect platform data
    const scope = await getPlatformScope(config);

    if (flags.format === OUTPUT_FORMATS.PLAIN) {
      const {
        httpService,
        httpPort,
        httpPortState,
        p2pService,
        p2pPort,
        p2pPortState,
        rpcService,
        tenderdash,
        drive,
      } = scope;

      plain['HTTP service'] = httpService || 'n/a';
      plain['HTTP port'] = `${httpPort} ${colors.portState(httpPortState)(httpPortState)}` || 'n/a';
      plain['P2P service'] = p2pService || 'n/a';
      plain['P2P port'] = `${p2pPort} ${colors.portState(p2pPortState)(p2pPortState)}` || 'n/a';
      plain['RPC service'] = rpcService || 'n/a';

      const { dockerStatus: tenderdashDockerStatus } = tenderdash;
      const { serviceStatus: tenderdashServiceStatus } = tenderdash;

      const { dockerStatus: driveDockerStatus } = drive;
      const { serviceStatus: driveServiceStatus } = drive;

      plain['Tenderdash Docker Status'] = colors.docker(tenderdashDockerStatus)(tenderdashDockerStatus) || 'n/a';
      plain['Tenderdash Service Status'] = colors.status(tenderdashServiceStatus)(tenderdashServiceStatus) || 'n/a';

      plain['Drive Docker Status'] = colors.docker(driveDockerStatus)(driveDockerStatus) || 'n/a';
      plain['Drive Service Status'] = colors.status(driveServiceStatus)(driveServiceStatus) || 'n/a';

      if (tenderdash.version) {
        const {
          version: tenderdashVersion,
          latestBlockHeight: platformBlockHeight,
          latestAppHash: platformLatestAppHash,
          peers: platformPeers,
          network: tenderdashNetwork,
        } = tenderdash;

        plain['Network'] = tenderdashNetwork || 'n/a';
        plain['Tenderdash Version'] = tenderdashVersion || 'n/a';
        plain['Block height'] = platformBlockHeight || 'n/a';
        plain['Peer count'] = platformPeers || 'n/a';
        plain['App hash'] = platformLatestAppHash || 'n/a';
      }

      return printObject(plain, flags.format);
    }

    return printObject(scope, flags.format);
  }
}

PlatformStatusCommand.description = 'Show Platform status details';

PlatformStatusCommand.flags = {
  ...ConfigBaseCommand.flags,
  format: Flags.string({
    description: 'display output format',
    default: OUTPUT_FORMATS.PLAIN,
    options: Object.values(OUTPUT_FORMATS),
  }),
};

module.exports = PlatformStatusCommand;
