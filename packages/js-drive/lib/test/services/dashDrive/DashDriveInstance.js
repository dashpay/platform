const DockerInstance = require('../docker/DockerInstance');

class DashDriveInstance extends DockerInstance {
  /**
   * Get Rpc port
   *
   * @return {int} port
   */
  getRpcPort() {
    return this.options.getRpcPort();
  }
}

module.exports = DashDriveInstance;
