const DockerInstanceOptions = require('../docker/DockerInstanceOptions');

class MongoDbInstanceOptions extends DockerInstanceOptions {
  constructor() {
    super();

    const mongoPort = this.getRandomPort(27001, 27998);
    this.mongo = {
      port: mongoPort,
      name: process.env.STORAGE_MONGODB_DB,
    };
    const container = {
      image: 'mongo:3.6',
      network: {
        name: 'dash_test_network',
        driver: 'bridge',
      },
      ports: [
        `${mongoPort}:27017`,
      ],
    };
    this.container = { ...this.container, ...container };
  }

  regeneratePorts() {
    const mongoPort = this.getRandomPort(27001, 27998);

    this.mongo.port = mongoPort;
    this.container.ports = [
      `${mongoPort}:27017`,
    ];

    return this;
  }
}

module.exports = MongoDbInstanceOptions;
