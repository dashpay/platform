const os = require('os');
const publicIp = require('public-ip');
const prettyMs = require('pretty-ms');
const prettyByte = require('pretty-bytes');

const ConfigBaseCommand = require('../../oclif/command/ConfigBaseCommand');
const getFormat = require('../../util/getFormat');
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

    printObject(outputRows, getFormat(flags));
  }
}

HostStatusCommand.description = 'Show host status details';

HostStatusCommand.flags = {
  ...ConfigBaseCommand.flags,
};

module.exports = HostStatusCommand;
