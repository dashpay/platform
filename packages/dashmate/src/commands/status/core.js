const chalk = require('chalk');

const {Flags} = require('@oclif/core');
const {OUTPUT_FORMATS} = require('../../constants');

const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');
const printObject = require('../../printers/printObject');

const providers = require('../../status/providers')

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
    outputStatusOverview
  ) {
    const statusOverview = await outputStatusOverview(config, ['core', 'external'])
    const latestVersion = await providers.github.release('dashpay/dash')
    const p2pPortState = await providers.mnowatch.checkPortStatus(config.get('core.p2p.port'))
    const remoteBlockHeight = await providers.insight(config.get('network')).status()
    const masternodeEnabled = config.get('core.masternode.enable')

    const {core} = statusOverview
    const { version, verificationProgress,
      blockHeight,
      headerHeight,
      peersCount,
      network,
      status,
      syncAsset,
      difficulty
    } = core

    const json = {
      version,
      network,
      latestVersion,
      status,
      syncAsset,
      peersCount,
      p2pService: `${config.get('externalIp')}:${config.get('core.p2p.port')}`,
      p2pPortState,
      rpcService: `127.0.0.1:${config.get('core.rpc.port')}`,
      blockHeight,
      headerHeight,
      difficulty,
      verificationProgress,
      masternode: {
        enabled: masternodeEnabled,
        sentinel: {
          status: null,
          version: null,
        }
      }
    }

    if (masternodeEnabled) {
      const {masternode} = await outputStatusOverview(config, ['masternode'])
      const {sentinelState, sentinelVersion} = masternode

      json.masternode.sentinel.status = sentinelState
      json.masternode.sentinel.version = sentinelVersion
    }

    if (flags.format === OUTPUT_FORMATS.PLAIN) {
      const plain = {
        'Version': version,
        'Latest version': latestVersion,
        'Network': network,
        'Status': status === 'syncing' ? `syncing ${(verificationProgress * 100).toFixed(2)}%` : status,
        'Sync asset': syncAsset,
        'Peer count': peersCount,
        'P2P service': `${config.get('externalIp')}:${config.get('core.p2p.port')}`,
        'P2P port': `${config.get('core.p2p.port')} ${p2pPortState}`,
        'RPC service': `127.0.0.1:${config.get('core.rpc.port')}`,
        'Block height': blockHeight,
        'Header height': headerHeight,
        'Verification Progress': `${verificationProgress}%`,
        'Remote Block Height': remoteBlockHeight || 'N/A',
        'Difficulty': difficulty,
      }

      // Apply colors
      switch (status) {
        case 'running':
          plain.Status = chalk.green(plain.Status);
          break;
        case 'syncing':
          plain.Status = chalk.yellow(plain.Status);
          break;
        default:
          plain.Status = chalk.red(plain.Status);
      }

      if (version === latestVersion) {
        plain.Version = chalk.green(plain.Version);
      } else if (version.match(/\d+.\d+/)[0] === latestVersion.match(/\d+.\d+/)[0]) {
        plain.Version = chalk.yellow(plain.Version);
      } else {
        plain.Version = chalk.red(plain.Version);
      }

      if (p2pPortState === 'OPEN') {
        plain["P2P port"] = chalk.green(plain["P2P port"]);
      } else {
        plain["P2P port"] = chalk.red(plain["P2P port"]);
      }

      if (blockHeight === headerHeight || blockHeight >= remoteBlockHeight) {
        plain["Block height"] = chalk.green(plain["Block height"]);
      } else if ((remoteBlockHeight - blockHeight) < 3) {
        plain["Block height"] = chalk.yellow(plain["Block height"]);
      } else {
        plain["Block height"] = chalk.red(plain["Block height"]);
      }

      if (masternodeEnabled) {
        plain['Sentinel version'] = json.masternode.sentinel.version;
        plain['Sentinel status'] = json.masternode.sentinel.status ? chalk.green('No errors') : chalk.red(sentinelState);
      }

      return printObject(plain, flags.format);
    }

    printObject(json, flags.format);
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
