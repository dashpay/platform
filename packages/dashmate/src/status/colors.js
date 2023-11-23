import chalk from 'chalk';
import { PortStateEnum } from './enums/portState.js';
import { ServiceStatusEnum } from './enums/serviceStatus.js';
import { DockerStatusEnum } from './enums/dockerStatus.js';

export default {
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
  docker: (status) => {
    if (!status) {
      return () => null;
    }
    switch (status) {
      case DockerStatusEnum.running:
        return chalk.green;
      default:
        return chalk.red;
    }
  },
  status: (status) => {
    if (!status) {
      return () => null;
    }
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
    if (!version) {
      return () => null;
    }
    if (version === latestVersion) {
      return chalk.green;
    }
    if (!latestVersion || version.match(/\d+.\d+/)[0] === latestVersion.match(/\d+.\d+/)[0]) {
      return chalk.yellow;
    }
    return chalk.red;
  },
  blockHeight: (blockHeight, headerHeight, remoteBlockHeight = 0) => {
    if (!blockHeight) {
      return () => null;
    }
    if (blockHeight === headerHeight && blockHeight >= remoteBlockHeight) {
      return chalk.green;
    }
    if (headerHeight - blockHeight < 3
      || (remoteBlockHeight > blockHeight && remoteBlockHeight - blockHeight < 3)) {
      return chalk.yellow;
    }
    return chalk.red;
  },
  poSePenalty: (poSePenalty, masternodeEnabled, evonodeEnabled) => {
    if (poSePenalty === 0) {
      return chalk.green;
    }
    if (poSePenalty < masternodeEnabled + evonodeEnabled * 4) {
      return chalk.yellow;
    }
    return chalk.red;
  },
};
