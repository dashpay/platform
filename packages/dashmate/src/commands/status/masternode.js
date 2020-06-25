const { table } = require('table');

const BaseCommand = require('../../oclif/command/BaseCommand');

const PRESETS = require('../../presets');

class MasternodeStatusCommand extends BaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {DockerCompose} dockerCompose
   * @return {Promise<void>}
   */
  async runWithDependencies(
    {
      preset,
    },
    flags,
    dockerCompose,
  ) {
    const rows = [];

    // Version
    const versionOutput = await dockerCompose.execCommand(
      preset,
      'core',
      'dashd --version',
    );

    rows.push(['Version', versionOutput.out.split('\n')[0]]);

    // Block count
    const blockCountOutput = await dockerCompose.execCommand(
      preset,
      'core',
      'dash-cli getblockcount',
    );

    rows.push(['Blocks', blockCountOutput.out.trim()]);

    const output = table(rows, {
      drawHorizontalLine: (index, size) => index === 0 || index === 1 || index === size,
    });

    // eslint-disable-next-line no-console
    console.log(output);
  }
}

MasternodeStatusCommand.description = 'Show masternode status details';

MasternodeStatusCommand.args = [{
  name: 'preset',
  required: true,
  description: 'preset to use',
  options: Object.values(PRESETS),
}];

module.exports = MasternodeStatusCommand;
