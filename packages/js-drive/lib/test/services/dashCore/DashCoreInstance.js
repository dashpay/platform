const DockerInstance = require('../docker/DockerInstance');

async function wait(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

class DashCoreInstance extends DockerInstance {
  /**
   * Create DashCore instance
   *
   * @param {Network} network
   * @param {Image} image
   * @param {Container} container
   * @param {RpcClient} RpcClient
   * @param {DashCoreInstanceOptions} options
   */
  constructor(network, image, container, RpcClient, options) {
    super(network, image, container, options);
    this.RpcClient = RpcClient;
    this.options = options;
  }

  /**
   * Start instance
   *
   * @return {Promise<void>}
   */
  async start() {
    await super.start();
    await this.initialization();
  }

  /**
   * Connect to another DashCore instance
   *
   * @param {Object} DashCoreInstance
   * @return {Promise<void>}
   */
  async connect(dashCoreInstance) {
    if (!this.isInitialized()) {
      throw new Error('Instance should be started before!');
    }

    const ip = dashCoreInstance.getIp();
    const port = dashCoreInstance.options.getDashdPort();
    await this.rpcClient.addNode(`${ip}:${port}`, 'add');
  }

  /**
   * Get RPC client
   *
   * @return {Object}
   */
  getApi() {
    if (!this.isInitialized()) {
      return {};
    }

    return this.rpcClient;
  }

  /**
   * @private
   *
   * @return {Promise<void>}
   */
  async initialization() {
    this.rpcClient = await this.createRpcClient();

    let nodeStarting = true;
    while (nodeStarting) {
      try {
        await this.rpcClient.getInfo();
        nodeStarting = false;
      } catch (error) {
        if (!this.isDashdLoading(error)) {
          throw error;
        }
        await wait(1000);
      }
    }
  }

  /**
   * @private
   *
   * @return {Boolean}
   */
  // eslint-disable-next-line class-methods-use-this
  isDashdLoading(error) {
    const messages = [
      'Loading',
      'Starting',
      'Verifying',
      'RPC',
      'Masternode cache is empty',
    ];
    const loading = messages.filter(message => error.message.includes(message));
    return !!loading.length;
  }

  /**
   * @private
   *
   * @return {Object} rpcClient
   */
  createRpcClient() {
    return new this.RpcClient({
      protocol: 'http',
      host: '127.0.0.1',
      port: this.options.getRpcPort(),
      user: this.options.getRpcUser(),
      pass: this.options.getRpcPassword(),
    });
  }
}

module.exports = DashCoreInstance;
