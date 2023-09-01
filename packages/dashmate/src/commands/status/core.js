/* eslint-disable quote-props */
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
   * @param {getCoreScope} getCoreScope
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    flags,
    dockerCompose,
    createRpcClient,
    config,
    getCoreScope,
  ) {
    const plain = {
      'Network': 'n/a',
      'Version': 'n/a',
      'Chain': 'n/a',
      'Docker Status': 'n/a',
      'Service Status': 'n/a',
      'Difficulty': 'n/a',
      'Latest version': 'n/a',
      'Sync asset': 'n/a',
      'Peer count': 'n/a',
      'P2P service': 'n/a',
      'P2P port': 'n/a',
      'RPC service': 'n/a',
      'Block height': 'n/a',
      'Header height': 'n/a',
      'Verification Progress': 'n/a',
      'Remote Block Height': 'n/a',
    };

    const scope = await getCoreScope(config);

    if (flags.format === OUTPUT_FORMATS.PLAIN) {
      const {
        version,
        network,
        chain,
        latestVersion,
        dockerStatus,
        serviceStatus,
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
      } = scope;

      const versionString = `${colors.version(version, latestVersion)(version) || 'n/a'} ${version && version !== latestVersion ? `(latest ${latestVersion})` : ''}`;

      plain.Network = network || 'n/a';
      plain.Version = versionString;
      plain.Chain = chain || 'n/a';
      plain['Docker Status'] = dockerStatus || 'n/a';
      plain['Service Status'] = serviceStatus || 'n/a';
      plain.Difficulty = difficulty || 'n/a';
      plain['Latest version'] = network || 'n/a';
      plain['Sync asset'] = syncAsset || 'n/a';
      plain['Peer count'] = peersCount || 'n/a';
      plain['P2P service'] = p2pService || 'n/a';
      plain['P2P port'] = colors.portState(p2pPortState)(p2pPortState) || 'n/a';
      plain['RPC service'] = rpcService || 'n/a';
      plain['Block height'] = colors.blockHeight(blockHeight, headerHeight, remoteBlockHeight)(blockHeight) || 'n/a';
      plain['Header height'] = headerHeight || 'n/a';
      plain['Verification Progress'] = verificationProgress
        ? `${(verificationProgress * 100).toFixed(2)}%` : 'n/a';
      plain['Remote Block Height'] = remoteBlockHeight || 'n/a';

      return printObject(plain, flags.format);
    }

    return printObject(scope, flags.format);
  }
}

CoreStatusCommand.description = 'Show Core status details';

CoreStatusCommand.flags = {
  ...ConfigBaseCommand.flags,
  format: Flags.string({
    description: 'display output format',
    default: OUTPUT_FORMATS.PLAIN,
    options: Object.values(OUTPUT_FORMATS),
  }),
};

module.exports = CoreStatusCommand;
