const DockerInstanceOptions = require('../docker/DockerInstanceOptions');

class DashCoreInstanceOptions extends DockerInstanceOptions {
  constructor() {
    super();

    const dashdPort = this.getRandomPort(20002, 29998);
    const rpcPort = this.getRandomPort(30002, 39998);

    this.dashd = {
      port: dashdPort,
    };
    this.zmqPorts = this.generateZmqPorts();
    this.rpc = {
      user: 'dashrpc',
      password: 'password',
      port: rpcPort,
    };
    const defaultPorts = [
      `${dashdPort}:${dashdPort}`,
      `${rpcPort}:${rpcPort}`,
    ];
    const container = {
      image: '103738324493.dkr.ecr.us-west-2.amazonaws.com/dashevo/dashcore:develop',
      network: {
        name: 'dash_test_network',
        driver: 'bridge',
      },
      ports: this.mergeWithZmqPorts(defaultPorts, this.zmqPorts),
      cmd: this.getCmd(),
    };
    this.container = { ...this.container, ...container };
  }

  regeneratePorts() {
    const dashdPort = this.getRandomPort(20002, 29998);
    const rpcPort = this.getRandomPort(30002, 39998);

    this.dashd = {
      port: dashdPort,
    };
    this.zmqPorts = this.generateZmqPorts();
    this.rpc.port = rpcPort;
    const defaultPorts = [
      `${dashdPort}:${dashdPort}`,
      `${rpcPort}:${rpcPort}`,
    ];
    this.container.ports = this.mergeWithZmqPorts(defaultPorts, this.zmqPorts);
    this.container.cmd = this.getCmd();

    return this;
  }

  getCmd() {
    const cmd = [
      'dashd',
      `-port=${this.dashd.port}`,
      `-rpcuser=${this.rpc.user}`,
      `-rpcpassword=${this.rpc.password}`,
      '-rpcallowip=0.0.0.0/0',
      '-regtest=1',
      '-keypool=1',
      `-rpcport=${this.rpc.port}`,
    ];
    return this.addZmqParamsToCmd(cmd, this.zmqPorts);
  }

  generateZmqPorts() {
    const rawtxPort = this.getRandomPort(40002, 40998);
    const rawtxlockPort = this.getRandomPort(41002, 41998);
    const hashblockPort = this.getRandomPort(42002, 42998);
    const hashtxPort = this.getRandomPort(43002, 43998);
    const hashtxlockPort = this.getRandomPort(44002, 44998);
    const rawblockPort = this.getRandomPort(45002, 45998);

    return {
      rawtx: rawtxPort,
      rawtxlock: rawtxlockPort,
      hashblock: hashblockPort,
      hashtx: hashtxPort,
      hashtxlock: hashtxlockPort,
      rawblock: rawblockPort,
    };
  }

  getDashdPort() {
    return this.dashd.port;
  }

  getZmqPorts() {
    return this.zmqPorts;
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

  // eslint-disable-next-line class-methods-use-this
  mergeWithZmqPorts(containerPorts, zmqPorts) {
    const ports = containerPorts.slice(0);
    for (const [, port] of Object.entries(zmqPorts)) {
      ports.push(`${port}:${port}`);
    }
    return ports;
  }

  // eslint-disable-next-line class-methods-use-this
  addZmqParamsToCmd(cmd, zmqPorts) {
    const command = cmd.slice(0);
    for (const [topicName, port] of Object.entries(zmqPorts)) {
      command.push(`-zmqpub${topicName}=tcp://0.0.0.0:${port}`);
    }
    return command;
  }
}

module.exports = DashCoreInstanceOptions;
