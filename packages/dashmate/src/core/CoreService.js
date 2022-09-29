class CoreService {
  /**
   *
   * @param {Config} config
   * @param {RpcClient} rpcClient
   * @param {Container} dockerContainer
   */
  constructor(config, rpcClient, dockerContainer) {
    this.config = config;
    this.rpcClient = rpcClient;
    this.dockerContainer = dockerContainer;
  }

  /**
   * @return {Config}
   */
  getConfig() {
    return this.config;
  }

  /**
   * Get RPC Client
   *
   * @return {RpcClient}
   */
  getRpcClient() {
    return this.rpcClient;
  }

  /**
   * Is Core running?
   *
   * @return {Promise<boolean>}
   */
  async isRunning() {
    const { State: { Status: status } } = await this.dockerContainer.inspect();

    return status === 'running';
  }

  /**
   * Stop Core service
   *
   * @return {Promise<boolean>}
   */
  async stop() {
    if (!await this.isRunning()) {
      return false;
    }

    await this.dockerContainer.stop();
    await this.dockerContainer.remove();

    return true;
  }
}

module.exports = CoreService;
