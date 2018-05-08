const DockerInstanceOptions = require('../docker/DockerInstanceOptions');

class DashDriveInstanceOptions extends DockerInstanceOptions {
  constructor({ envs }) {
    super();

    const rootPath = process.cwd();
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
      volumes: [
        `${rootPath}/lib:/usr/src/app/lib`,
        `${rootPath}/scripts:/usr/src/app/scripts`,
        `${rootPath}/package.json:/usr/src/app/package.json`,
      ],
    };
    this.container = { ...this.container, ...container };
  }
}

module.exports = DashDriveInstanceOptions;
