const DockerInstanceOptions = require('../docker/DockerInstanceOptions');

class DashDriveInstanceOptions extends DockerInstanceOptions {
  constructor({ envs }) {
    super();

    const container = {
      image: '103738324493.dkr.ecr.us-west-2.amazonaws.com/dashevo/dashdrive',
      envs,
      cmd: [
        'npm',
        'run',
        'sync',
      ],
      network: {
        name: 'dash_test_network',
        driver: 'bridge',
      },
    };
    this.container = { ...this.container, ...container };
  }
}

module.exports = DashDriveInstanceOptions;
