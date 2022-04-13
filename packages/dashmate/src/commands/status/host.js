const os = require('os');
const publicIp = require('public-ip');
const prettyMs = require('pretty-ms');
const prettyByte = require('pretty-bytes');

const { Flags } = require('@oclif/core');
const { OUTPUT_FORMATS } = require('../../constants');

const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');
const printObject = require('../../printers/printObject');

class HostStatusCommand extends ConfigBaseCommand {
  /**
   * @return {Promise<void>}
   */
  async runWithDependencies(args, flags) {
    const outputRows = {
      Hostname: os.hostname(),
      Uptime: prettyMs(os.uptime() * 1000),
      Platform: os.platform(),
      Arch: os.arch(),
      Username: os.userInfo().username,
      Diskfree: 0, // Waiting for feature: https://github.com/nodejs/node/pull/31351
      Memory: `${prettyByte(os.totalmem())} / ${prettyByte(os.freemem())}`,
      CPUs: os.cpus().length,
      IP: await publicIp.v4(),
    };

    printObject(outputRows, flags.format);
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
