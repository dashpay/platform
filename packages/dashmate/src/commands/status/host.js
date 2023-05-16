const { Flags } = require('@oclif/core');
const { OUTPUT_FORMATS } = require('../../constants');

const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');
const printObject = require('../../printers/printObject');

class HostStatusCommand extends ConfigBaseCommand {
  /**
   * @return {Promise<void>}
   */
  async runWithDependencies(args, flags, getHostScope) {
    const plain = {
      Hostname: 'n/a',
      Uptime: 'n/a',
      Platform: 'n/a',
      Arch: 'n/a',
      Username: 'n/a',
      Memory: 'n/a',
      CPUs: 'n/a',
      IP: 'n/a',
    };

    const scope = await getHostScope();

    if (flags.format === OUTPUT_FORMATS.PLAIN) {
      const {
        hostname, uptime, platform, arch,
        username, memory, cpus, ip,
      } = scope;

      plain.Hostname = hostname || 'n/a';
      plain.Uptime = uptime || 'n/a';
      plain.Platform = platform || 'n/a';
      plain.Arch = arch || 'n/a';
      plain.Username = username || 'n/a';
      plain.Memory = memory || 'n/a';
      plain.CPUs = cpus || 'n/a';
      plain.IP = ip || 'n/a';

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
