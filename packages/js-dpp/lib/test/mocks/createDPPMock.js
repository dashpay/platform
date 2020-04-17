/**
 * @param {Sandbox} sinonSandbox
 *
 * @returns {DashPlatformProtocol}
 */
module.exports = function createDPPMock(sinonSandbox) {
  const dataContract = {
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

  const stateTransition = {
    createFromObject: sinonSandbox.stub(),
    createFromSerialized: sinonSandbox.stub(),
    validate: sinonSandbox.stub(),
    validateStructure: sinonSandbox.stub(),
    validateData: sinonSandbox.stub(),
    apply: sinonSandbox.stub(),
  };

  const identity = {
    create: sinonSandbox.stub(),
    createFromObject: sinonSandbox.stub(),
    createFromSerialized: sinonSandbox.stub(),
    validate: sinonSandbox.stub(),
  };

  return {
    dataContract,
    document,
    stateTransition,
    identity,
    getOwnerId: sinonSandbox.stub(),
    setOwnerId: sinonSandbox.stub(),
    getDataContract: sinonSandbox.stub(),
    setDataContract: sinonSandbox.stub(),
    getStateRepository: sinonSandbox.stub(),
  };
};
