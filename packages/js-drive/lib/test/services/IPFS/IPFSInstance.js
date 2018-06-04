const DockerInstance = require('../docker/DockerInstance');

function wait(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

class IPFSInstance extends DockerInstance {
  /**
   * Create IPFS instance
   *
   * @param {Network} network
   * @param {Image} image
   * @param {Container} container
   * @param {IpfsApi} IpfsApi
   * @param {IPFSInstanceOptions} options
   */
  constructor(network, image, container, IpfsApi, options) {
    super(network, image, container, options);
    this.IpfsApi = IpfsApi;
    this.options = options;
  }

  /**
   * Start IPFS instance
   *
   * @returns {Promise<void>}
   */
  async start() {
    await super.start();
    await this.initialize();
  }

  /**
   * Connect to another IPFS instance
   *
   * @param {IPFSInstance} ipfsInstance
   * @returns {Promise<void>}
   */
  async connect(ipfsInstance) {
    const externalIpfs = ipfsInstance.getApi();
    const externalIpfsId = await externalIpfs.id();
    const internalIpfs = this.getApi();
    const addr = `/ip4/${ipfsInstance.getIp()}/tcp/${this.options.getIpfsExposedPort()}/ipfs/${externalIpfsId.id}`;
    await internalIpfs.swarm.connect(addr);
  }

  /**
   * Get IPFS address
   *
   * @returns {string}
   */
  getIpfsAddress() {
    return `/ip4/${this.getIp()}/tcp/${this.options.getIpfsInternalPort()}`;
  }

  /**
   * Get IPFS client
   *
   * @returns {ipfsClient}
   */
  getApi() {
    return this.ipfsClient;
  }

  /**
   * @private
   *
   * @returns {Promise<void>}
   */
  async initialize() {
    const address = `/ip4/127.0.0.1/tcp/${this.options.getIpfsExposedPort()}`;
    this.ipfsClient = await this.IpfsApi(address);

    let starting = true;
    while (starting) {
      try {
        await this.ipfsClient.swarm.peers();
        starting = false;
      } catch (error) {
        if (error && !this.isDaemonLoading(error)) {
          throw error;
        }
        await wait(1000);
      }
    }
  }

  /**
   * @private
   *
   * @param error
   * @returns {boolean}
   */
  // eslint-disable-next-line class-methods-use-this
  isDaemonLoading(error) {
    const messages = [
      'socket hang up',
      'ECONNRESET',
    ];
    const loading = messages.filter(message => error.message.includes(message));
    return !!loading.length;
  }
}

module.exports = IPFSInstance;
