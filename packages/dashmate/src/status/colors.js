const PortStateEnum = require("../enums/portState");

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
  }
}
