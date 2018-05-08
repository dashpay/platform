const Docker = require('dockerode');

class Container {
  /**
   * Create Docker container
   *
   * @param {Network} network
   * @param {Image} image
   * @param {Object} options
   * @param {Array} options.cmd
   * @param {Array} options.envs
   * @param {Array} options.ports
   * @param {Array} options.volumes
   */
  constructor(network, image, options) {
    this.docker = new Docker();
    this.network = network;
    this.image = image;
    this.cmd = options.cmd;
    this.envs = options.envs;
    this.ports = options.ports;
    this.volumes = options.volumes;
    this.container = null;
    this.containerIp = null;
    this.initialized = false;
  }

  /**
   * Start container
   *
   * @return {Promise<void>}
   */
  async start() {
    if (this.initialized) {
      return;
    }
    if (this.container) {
      await this.container.start();
      this.initialized = true;
      return;
    }

    this.container = await this.create();
    const { NetworkSettings: { Networks } } = await this.container.inspect();
    this.containerIp = Networks[this.network].IPAddress;

    this.initialized = true;
  }

  /**
   * Stop container
   *
   * @return {Promise<void>}
   */
  async stop() {
    if (!this.initialized) {
      return;
    }
    await this.container.stop();
    this.initialized = false;
  }

  /**
   * Remove container
   *
   * @return {Promise<void>}
   */
  async remove() {
    if (!this.initialized) {
      return;
    }
    await this.container.stop();
    await this.container.remove();
    this.initialized = false;
  }

  /**
   * Retrieve container details
   *
   * @return {Promise<details>}
   */
  async details() {
    return this.container.inspect();
  }

  /**
   * Get container IP
   *
   * @return {String}
   */
  getIp() {
    if (!this.initialized) {
      return null;
    }
    return this.containerIp;
  }

  /**
   * Set container options
   *
   * @param {Object} options
   * @param {Array} options.cmd
   * @param {Array} options.envs
   * @param {Array} options.ports
   * @param {Array} options.volumes
   *
   * @return {void}
   */
  setOptions(options) {
    this.cmd = options.cmd;
    this.envs = options.envs;
    this.ports = options.ports;
    this.volumes = options.volumes;
  }

  /**
   * Check if container is initialized
   *
   * @return {Boolean}
   */
  isInitialized() {
    return this.container && this.initialized;
  }

  /**
   * @private
   *
   * @return {Promise<void>}
   */
  async create() {
    const ports = Object.entries(this.ports).map(([, value]) => value);
    const ExposedPorts = this.createExposedPorts(ports);
    const PortBindings = this.createPortBindings(ports);

    const EndpointsConfig = {};
    EndpointsConfig[this.network] = {};

    const Volumes = this.createVolumes(this.volumes);
    const Binds = this.volumes;

    const params = {
      Image: this.image,
      Env: this.envs,
      ExposedPorts,
      Volumes,
      HostConfig: {
        Binds,
        PortBindings,
      },
      NetworkingConfig: {
        EndpointsConfig,
      },
    };
    if (this.cmd) {
      params.Cmd = this.cmd;
    }

    const container = await this.docker.createContainer(params);
    try {
      await container.start();
    } catch (error) {
      await this.removeContainer(container);
      throw error;
    }

    return container;
  }

  /**
   * @private
   *
   * @return {Promise<void>}
   */
  async removeContainer(container) {
    await container.remove();
    this.initialized = false;
  }

  /**
   * @private
   *
   * @return {Object}
   */
  // eslint-disable-next-line class-methods-use-this
  createExposedPorts(ports) {
    return ports.reduce((exposedPorts, port) => {
      const result = exposedPorts;
      const [hostPort] = port.split(':');
      result[`${hostPort}/tcp`] = {};
      return result;
    }, {});
  }

  /**
   * @private
   *
   * @return {Object}
   */
  // eslint-disable-next-line class-methods-use-this
  createPortBindings(ports) {
    return ports.reduce((portBindings, port) => {
      const result = portBindings;
      const [hostPort, containerPort] = port.split(':');
      result[`${containerPort}/tcp`] = [{ HostPort: hostPort.toString() }];
      return result;
    }, {});
  }

  /**
   * @private
   *
   * @return {Object}
   */
  // eslint-disable-next-line class-methods-use-this
  createVolumes(volumes) {
    return volumes.reduce((mountPoints, volume) => {
      const result = mountPoints;
      const [, containerPath] = volume.split(':');
      result[containerPath] = {};
      return result;
    }, {});
  }
}

module.exports = Container;
