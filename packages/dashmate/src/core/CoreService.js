class CoreService {
  /**
   *
   * @param {RpcClient} rpcClient
   * @param {Container} dockerContainer
   */
  constructor(rpcClient, dockerContainer) {
    this.rpcClient = rpcClient;
    this.dockerContainer = dockerContainer;
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
