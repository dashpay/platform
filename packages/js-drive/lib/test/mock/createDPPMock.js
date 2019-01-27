/**
 * @param {Sandbox} sinonSandbox
 */
function createDPPMock(sinonSandbox) {
  const packet = {
    createFromSerialized: sinonSandbox.stub(),
  };

  return {
    packet,
  };
}

module.exports = createDPPMock;
