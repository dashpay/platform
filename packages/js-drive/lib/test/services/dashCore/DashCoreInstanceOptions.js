const DockerInstanceOptions = require('../docker/DockerInstanceOptions');

class DashCoreInstanceOptions extends DockerInstanceOptions {
  constructor() {
    super();

    const dashdPort = this.getRandomPort(20002, 29998);
    const rpcPort = this.getRandomPort(30002, 39998);
    const zmqPort = this.getRandomPort(40002, 49998);

    this.dashd = {
      port: dashdPort,
    };
    this.zmqEndpoints = {
      hashblock: `tcp://127.0.0.1:${zmqPort}`,
    };
    this.rpc = {
      user: 'dashrpc',
      password: 'password',
      port: rpcPort,
    };
    const container = {
      image: '103738324493.dkr.ecr.us-west-2.amazonaws.com/dashevo/dashcore:develop',
      network: {
        name: 'dash_test_network',
        driver: 'bridge',
      },
      ports: [
        `${dashdPort}:${dashdPort}`,
        `${rpcPort}:${rpcPort}`,
        `${zmqPort}:${zmqPort}`,
      ],
      cmd: this.getCmd(),
    };
    this.container = { ...this.container, ...container };
  }

  regeneratePorts() {
    const dashdPort = this.getRandomPort(20002, 29998);
    const rpcPort = this.getRandomPort(30002, 39998);
    const zmqPort = this.getRandomPort(40002, 49998);

    this.dashd = {
      port: dashdPort,
    };
    this.zmqEndpoints = {
      hashblock: `tcp://127.0.0.1:${zmqPort}`,
    };
    this.rpc.port = rpcPort;
    this.container.ports = [
      `${dashdPort}:${dashdPort}`,
      `${rpcPort}:${rpcPort}`,
      `${zmqPort}:${zmqPort}`,
    ];
    this.container.cmd = this.getCmd();

    return this;
  }

  getCmd() {
    return [
      'dashd',
      `-port=${this.dashd.port}`,
      `-rpcuser=${this.rpc.user}`,
      `-rpcpassword=${this.rpc.password}`,
      '-rpcallowip=0.0.0.0/0',
      '-regtest=1',
      '-keypool=1',
      `-rpcport=${this.rpc.port}`,
      `-zmqpubhashblock=${this.zmqEndpoints.hashblock}`,
    ];
  }

  getDashdPort() {
    return this.dashd.port;
  }

  getZmqSockets() {
    return this.zmqEndpoints;
  }

  getRpcPort() {
    return this.rpc.port;
  }

  getRpcPassword() {
    return this.rpc.password;
  }

  getRpcUser() {
    return this.rpc.user;
  }
}

module.exports = DashCoreInstanceOptions;
