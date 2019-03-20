/**
 * @param {Sandbox} sinonSandbox
 *
 * @returns {DashPlatformProtocol}
 */
module.exports = function createDPPMock(sinonSandbox) {
  const contract = {
    create: sinonSandbox.stub(),
    createFromObject: sinonSandbox.stub(),
    createFromSerialized: sinonSandbox.stub(),
    validate: sinonSandbox.stub(),
  };

  const document = {
    create: sinonSandbox.stub(),
    createFromObject: sinonSandbox.stub(),
    createFromSerialized: sinonSandbox.stub(),
    validate: sinonSandbox.stub(),
  };

  const packet = {
    create: sinonSandbox.stub(),
    createFromObject: sinonSandbox.stub(),
    createFromSerialized: sinonSandbox.stub(),
    validate: sinonSandbox.stub(),
    verify: sinonSandbox.stub(),
  };

  const packetHeader = {
    create: sinonSandbox.stub(),
    createFromObject: sinonSandbox.stub(),
    createFromSerialized: sinonSandbox.stub(),
    validate: sinonSandbox.stub(),
  };

  return {
    contract,
    document,
    packet,
    packetHeader,
    getUserId: sinonSandbox.stub(),
    setUserId: sinonSandbox.stub(),
    getDPContract: sinonSandbox.stub(),
    setDPContract: sinonSandbox.stub(),
    getDataProvider: sinonSandbox.stub(),
    setDataProvider: sinonSandbox.stub(),
  };
};
