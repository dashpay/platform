const { table } = require('table');

const BaseCommand = require('../../oclif/command/BaseCommand');

class MasternodeStatusCommand extends BaseCommand {
  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {DockerCompose} dockerCompose
   * @param {Config} config
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    flags,
    dockerCompose,
    config,
  ) {
    const rows = [];

    // Version
    const versionOutput = await dockerCompose.execCommand(
      config.toEnvs(),
      'core',
      'dashd --version',
    );

    rows.push(['Version', versionOutput.out.split('\n')[0]]);

    // Block count
    const blockCountOutput = await dockerCompose.execCommand(
      config.toEnvs(),
      'core',
      'dash-cli getblockcount',
    );

    rows.push(['Blocks', blockCountOutput.out.trim()]);

    const output = table(rows, { singleLine: true });

    // eslint-disable-next-line no-console
    console.log(output);
  }
}

MasternodeStatusCommand.description = 'Show masternode status details';

MasternodeStatusCommand.flags = {
  ...BaseCommand.flags,
};

module.exports = MasternodeStatusCommand;
