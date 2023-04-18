const { Flags } = require('@oclif/core');
const { OUTPUT_FORMATS } = require('../../constants');

const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');
const printObject = require('../../printers/printObject');

class HostStatusCommand extends ConfigBaseCommand {
  /**
   * @return {Promise<void>}
   */
  async runWithDependencies(args, flags, getHostScope) {
    const scope = await getHostScope();

    if (flags.format === OUTPUT_FORMATS.PLAIN) {
      const {
        hostname, uptime, platform, arch,
        username, memory, cpus, ip,
      } = scope;

      const plain = {
        Hostname: hostname,
        Uptime: uptime,
        Platform: platform,
        Arch: arch,
        Username: username,
        Memory: memory,
        CPUs: cpus,
        IP: ip,
      };

      return printObject(plain, flags.format);
    }

    return printObject(scope, flags.format);
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
