class DockerInstanceOptions {
  constructor() {
    this.container = {
      network: {
        name: null,
        driver: null,
      },
      image: null,
      cmd: [],
      volumes: [],
      envs: [],
      ports: [],
      labels: {
        testHelperName: 'DashTestContainer',
      },
    };
  }

  regeneratePorts() {
    return this;
  }

  getContainerImageName() {
    return this.container.image;
  }

  getContainerOptions() {
    return this.container;
  }

  getContainerNetworkOptions() {
    return this.container.network;
  }

  // eslint-disable-next-line class-methods-use-this
  getRandomPort(min, max) {
    return Math.floor((Math.random() * ((max - min) + 1)) + min);
  }
}

module.exports = DockerInstanceOptions;
