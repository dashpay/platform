const chalk = require('chalk');
const PortStateEnum = require('../enums/portState');
const ServiceStatusEnum = require('../enums/serviceStatus');

module.exports = {

  /**
   *
   * @param portStateEnum {PortStateEnum}
   */
  portState: (portStateEnum) => {
    if (portStateEnum === PortStateEnum.OPEN) {
      return chalk.green;
    }
    return chalk.red;
  },
  status: (status) => {
    switch (status) {
      case ServiceStatusEnum.up:
        return chalk.green;
      case ServiceStatusEnum.syncing:
      case ServiceStatusEnum.wait_for_core:
        return chalk.yellow;
      default:
        return chalk.red;
    }
  },
  version: (version, latestVersion) => {
    if (version === latestVersion) {
      return chalk.green;
    } if (!latestVersion || version.match(/\d+.\d+/)[0] === latestVersion.match(/\d+.\d+/)[0]) {
      return chalk.yellow;
    }
    return chalk.red;
  },
  blockHeight: (blockHeight, headerHeight, remoteBlockHeight) => {
    if ((!remoteBlockHeight && blockHeight === headerHeight)
      || blockHeight >= remoteBlockHeight) {
      return chalk.green;
    } if ((!remoteBlockHeight && (headerHeight - blockHeight < 3))
      || (remoteBlockHeight - blockHeight) < 3) {
      return chalk.yellow;
    }
    return chalk.red;
  },
  poSePenalty: (poSePenalty, enabledCount) => {
    if (poSePenalty === 0) {
      return chalk.green;
    } if (poSePenalty < enabledCount) {
      return chalk.yellow;
    }
    return chalk.red;
  },
};
