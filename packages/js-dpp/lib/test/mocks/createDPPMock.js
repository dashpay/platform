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
  };

  return {
    dataContract,
    document,
    stateTransition,
    getUserId: sinonSandbox.stub(),
    setUserId: sinonSandbox.stub(),
    getDataContract: sinonSandbox.stub(),
    setDataContract: sinonSandbox.stub(),
    getDataProvider: sinonSandbox.stub(),
    setDataProvider: sinonSandbox.stub(),
  };
};
