const { protocolVersion } = require('../../protocolVersion');

/**
 * @param {Sandbox} [sinonSandbox]
 *
 * @returns {DashPlatformProtocol}
 */
module.exports = function createDPPMock(sinonSandbox = undefined) {
  // in simplier cases when you do not have acccess
  // to Sinon sandbox return a simplified version of DPP
  // with some predefined behaviour
  if (!sinonSandbox) {
    return {
      getProtocolVersion: () => protocolVersion,
    };
  }

  const dataContract = {
    create: sinonSandbox.stub(),
    createFromObject: sinonSandbox.stub(),
    createFromBuffer: sinonSandbox.stub(),
    validate: sinonSandbox.stub(),
  };

  const document = {
    create: sinonSandbox.stub(),
    createFromObject: sinonSandbox.stub(),
    createFromBuffer: sinonSandbox.stub(),
    validate: sinonSandbox.stub(),
  };

  const stateTransition = {
    createFromObject: sinonSandbox.stub(),
    createFromBuffer: sinonSandbox.stub(),
    validate: sinonSandbox.stub(),
    validateStructure: sinonSandbox.stub(),
    validateData: sinonSandbox.stub(),
    apply: sinonSandbox.stub(),
  };

  const identity = {
    create: sinonSandbox.stub(),
    createFromObject: sinonSandbox.stub(),
    createFromBuffer: sinonSandbox.stub(),
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
    getProtocolVersion: sinonSandbox.stub().returns(protocolVersion),
  };
};
