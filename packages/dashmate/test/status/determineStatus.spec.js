const sinon = require('sinon');
const determineStatus = require('../../src/status/determineStatus');
const ContainerIsNotPresentError = require('../../src/docker/errors/ContainerIsNotPresentError');

describe('determineStatus', () => {
  let dockerComposeMock;

  const config = { toEnvs: sinon.stub() };

  beforeEach(async () => {
    dockerComposeMock = { inspectService: sinon.stub() };
  });

  it('should return status from Docker', async () => {
    const mockDockerStatus = 'mock_status';

    dockerComposeMock.inspectService.resolves({ State: { Status: mockDockerStatus } });

    const status = await determineStatus(dockerComposeMock, config, 'sample_service');

    expect(status).to.equal(mockDockerStatus);
  });

  it('should return not_started in case ContainerIsNotPresent', async () => {
    const error = new ContainerIsNotPresentError();

    dockerComposeMock.inspectService.throws(error);

    const status = await determineStatus(dockerComposeMock, config, 'sample_service');

    expect(status).to.equal('not_started');
  });

  it('should throw in case unknown error', async () => {
    const error = new Error('unknown');

    dockerComposeMock.inspectService.throws(error);

    try {
      await determineStatus(dockerComposeMock, config, 'sample_service');

      expect.fail('should throw an error');
    } catch (e) {
      expect(e.message).to.equal('unknown');
    }
  });
});
