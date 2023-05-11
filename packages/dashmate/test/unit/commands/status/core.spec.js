const MasternodeSyncAssetEnum = require('../../../../src/status/enums/masternodeSyncAsset');
const ServiceStatusEnum = require('../../../../src/status/enums/serviceStatus');
const DockerStatusEnum = require('../../../../src/status/enums/dockerStatus');
const getCoreScopeFactory = require('../../../../src/status/scopes/core');
const determineStatus = require('../../../../src/status/determineStatus');
const providers = require('../../../../src/status/providers');
const ServiceIsNotRunningError = require('../../../../src/commands/status/core');

describe('CoreStatusCommand unit test', () => {
  describe('CoreStatusCommand', async () => {
    it('should just work', async () => {
      const statusCommand = new CoreStatusCommand()

      const args = {}
      const flags = {}
      const dockerCompose = {}
      const createRpcClient = {}
      const config = {}
      const getCoreScope = {}

      const result = await statusCommand.runWithDependencies(args,
        flags,
        dockerCompose,
        createRpcClient,
        config,
        getCoreScope)

    })
  })
})
