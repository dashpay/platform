const DockerInstanceOptions = require('../docker/DockerInstanceOptions');

class DashDriveInstanceOptions extends DockerInstanceOptions {
  constructor({ envs }) {
    super();

    const rootPath = process.cwd();
    const rpcPort = this.getRandomPort(50002, 59998);
    this.rpc = {
      port: rpcPort,
    };
    const container = {
      image: '103738324493.dkr.ecr.us-west-2.amazonaws.com/dashevo/dashdrive',
      envs,
      cmd: ['sh', '-c', 'cd / && npm i && cd /usr/src/app && npm run sync & npm run api'],
      network: {
        name: 'dash_test_network',
        driver: 'bridge',
      },
      ports: [
        `${rpcPort}:6000`,
      ],
      volumes: [
        `${rootPath}/lib:/usr/src/app/lib`,
        `${rootPath}/scripts:/usr/src/app/scripts`,
        `${rootPath}/package.json:/usr/src/app/package.json`,
        `${rootPath}/package-lock.json:/usr/src/app/package-lock.json`,
        `${rootPath}/package.json:/package.json`,
        `${rootPath}/package-lock.json:/package-lock.json`,
      ],
    };
    this.container = { ...this.container, ...container };
  }

  regeneratePorts() {
    const rpcPort = this.getRandomPort(50002, 59998);

    this.rpc.port = rpcPort;
    this.container.ports = [
      `${rpcPort}:6000`,
    ];

    return this;
  }

  getRpcPort() {
    return this.rpc.port;
  }
}

module.exports = DashDriveInstanceOptions;
