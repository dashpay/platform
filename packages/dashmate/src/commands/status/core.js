const chalk = require('chalk');

const { Flags } = require('@oclif/core');
const { OUTPUT_FORMATS } = require('../../constants');

const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');
const printObject = require('../../printers/printObject');

const colors = require('../../status/colors');

class CoreStatusCommand extends ConfigBaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {DockerCompose} dockerCompose
   * @param {createRpcClient} createRpcClient
   * @param {Config} config
   * @param statusProvider statusProvider
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    flags,
    dockerCompose,
    createRpcClient,
    config,
    statusProvider,
  ) {
    const scope = await statusProvider.getCoreScope();

    if (flags.format === OUTPUT_FORMATS.PLAIN) {
      const {
        version,
        network,
        chain,
        latestVersion,
        status,
        syncAsset,
        peersCount,
        p2pService,
        p2pPortState,
        rpcService,
        blockHeight,
        remoteBlockHeight,
        headerHeight,
        difficulty,
        verificationProgress,
        masternode,
      } = scope;

      const plain = {
        Version: colors.status(version, latestVersion)(version),
        'Latest version': latestVersion,
        Network: network,
        Chain: chain,
        Status: colors.status(status)(status),
        'Sync asset': syncAsset,
        'Peer count': peersCount,
        'P2P service': p2pService,
        'P2P port': colors.portState(p2pPortState)(p2pPortState),
        'RPC service': rpcService,
        'Block height': colors.blockHeight(blockHeight, headerHeight, remoteBlockHeight)(blockHeight),
        'Header height': headerHeight,
        'Verification Progress': `${verificationProgress * 100}%`,
        'Remote Block Height': remoteBlockHeight || 'N/A',
        Difficulty: difficulty,
      };

      if (masternode.enabled) {
        plain['Sentinel version'] = masternode.sentinelVersion;
        plain['Sentinel status'] = masternode.sentinel.status ? chalk.green('No errors') : chalk.red(masternode.sentinelState);
      }

      return printObject(plain, flags.format);
    }

    printObject(scope, flags.format);
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
