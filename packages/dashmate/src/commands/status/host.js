const {Flags} = require('@oclif/core');
const {OUTPUT_FORMATS} = require('../../constants');

const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');
const printObject = require('../../printers/printObject');

class HostStatusCommand extends ConfigBaseCommand {
  /**
   * @return {Promise<void>}
   */
  async runWithDependencies(args, flags, outputStatusOverview, config) {
    const status = await outputStatusOverview(config, ['host'])

    const json = status.host

    if (flags.format === OUTPUT_FORMATS.PLAIN) {
      const {
        hostname, uptime, platform, arch,
        username, diskFree, memory, cpus, ip
      } = json

      const plain = {
        Hostname: hostname,
        Uptime: uptime,
        Platform: platform,
        Arch: arch,
        Username: username,
        Diskfree: diskFree,
        Memory: memory,
        CPUs: cpus,
        IP: ip,
      }

      return printObject(plain, flags.format);
    }

    printObject(json, flags.format);
  }
}

HostStatusCommand.description = 'Show host status details';

HostStatusCommand.flags = {
  ...ConfigBaseCommand.flags,
  format: Flags.string({
    description: 'display output format',
    default: OUTPUT_FORMATS.PLAIN,
    options: Object.values(OUTPUT_FORMATS),
  }),
};

module.exports = HostStatusCommand;
