const chalk = require('chalk')
const PortStateEnum = require("../enums/portState");
const ServiceStatusEnum = require("../enums/serviceStatus");

module.exports = {

  /**
   *
   * @param portStateEnum {PortStateEnum}
   */
  portState: (portStateEnum) => {
    if (portStateEnum === PortStateEnum.OPEN) {
      return chalk.green
    } else {
      return chalk.red
    }
  },
  status: (status) => {
    switch (status) {
      case ServiceStatusEnum.running:
        return chalk.green
      case ServiceStatusEnum.syncing:
      case ServiceStatusEnum.wait_for_core:
        return chalk.yellow
      default:
        return chalk.red
    }
  },
  version: (version, latestVersion) => {
    if (version === latestVersion) {
      return chalk.green;
    } else if (version.match(/\d+.\d+/)[0] === latestVersion.match(/\d+.\d+/)[0]) {
      return chalk.yellow
    } else {
      return chalk.red
    }
  },
  blockHeight: (blockHeight, headerHeight, remoteBlockHeight) => {
    if (blockHeight === headerHeight || blockHeight >= remoteBlockHeight) {
      return chalk.green
    } else if ((remoteBlockHeight - blockHeight) < 3) {
      return chalk.yellow
    } else {
      return chalk.green
    }
  },
  poSePenalty: (poSePenalty, enabledCount) => {
    if (poSePenalty === 0) {
      return chalk.green;
    } else if (poSePenalty < enabledCount) {
      return chalk.yellow;
    } else {
      return chalk.red;
    }
  }
}
