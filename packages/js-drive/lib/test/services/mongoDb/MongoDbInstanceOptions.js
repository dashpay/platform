const DockerInstanceOptions = require('../docker/DockerInstanceOptions');

class MongoDbInstanceOptions extends DockerInstanceOptions {
  constructor() {
    super();

    const container = {
      image: 'mongo:3.6',
      network: {
        name: 'dash_test_network',
        driver: 'bridge',
      },
    };
    this.container = { ...this.container, ...container };
  }
}

module.exports = MongoDbInstanceOptions;
