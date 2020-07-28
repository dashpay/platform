const { Listr } = require('listr2');

const { PrivateKey } = require('@dashevo/dashcore-lib');

const BaseCommand = require('../oclif/command/BaseCommand');
const MuteOneLineError = require('../oclif/errors/MuteOneLineError');

const PRESETS = require('../presets');

const masternodeDashAmount = require('../core/masternodeDashAmount');

class RegisterCommand extends BaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {registerMasternodeTask} registerMasternodeTask
   * @return {Promise<void>}
   */
  async runWithDependencies(
    {
      preset, port: coreP2pPort, 'funding-private-key': fundingPrivateKeyString, 'external-ip': externalIp,
    },
    flags,
    registerMasternodeTask,
  ) {
    const network = preset;

    const fundingPrivateKey = new PrivateKey(
      fundingPrivateKeyString,
      network,
    );

    const fundingAddress = fundingPrivateKey.toAddress(network).toString();

    const tasks = new Listr([
      {
        title: `Register masternode using ${preset} preset`,
        task: () => registerMasternodeTask(preset),
      },
    ],
    {
      rendererOptions: {
        clearOutput: false,
        collapse: false,
        showSubtasks: true,
      },
    });

    try {
      await tasks.run({
        fundingAddress,
        fundingPrivateKeyString,
        externalIp,
        coreP2pPort,
        network,
      });
    } catch (e) {
      throw new MuteOneLineError(e);
    }
  }
}

RegisterCommand.description = `Register masternode
...
Register masternode using predefined presets
`;

RegisterCommand.args = [{
  name: 'preset',
  required: true,
  description: 'preset to use',
  options: Object.values(PRESETS),
}, {
  name: 'funding-private-key',
  required: true,
  description: `private key with more than ${masternodeDashAmount} dash for funding collateral`,
}, {
  name: 'external-ip',
  required: true,
  description: 'masternode external IP',
}, {
  name: 'port',
  required: true,
  description: 'masternode P2P port',
}];

module.exports = RegisterCommand;
