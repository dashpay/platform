const DashPlatformProtocol = require('@dashevo/dpp/lib/DashPlatformProtocol');
const protocolVersion = require('@dashevo/dpp/lib/version/protocolVersion');
const getChainAssetLockProofFixture = require('@dashevo/dpp/lib/test/fixtures/getChainAssetLockProofFixture');
const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');

describe('DashPlatformProtocol', () => {
  let dpp;

  beforeEach(async () => {
    dpp = new DashPlatformProtocol({});
    await dpp.initialize();
  });

  it('should propagate protocol version to factories', async () => {
    let dataContract = dpp.dataContract.create(generateRandomIdentifier(), {
      niceDocument: {
        type: 'object',
        properties: {
          name: {
            type: 'string',
          },
        },
        required: ['$createdAt'],
        additionalProperties: false,
      },
    });
    const document = dpp.document.create(dataContract, generateRandomIdentifier(), 'niceDocument', {});
    let identity = dpp.identity.create(getChainAssetLockProofFixture(), []);

    expect(dataContract.protocolVersion).to.equal(protocolVersion.latestVersion);
    expect(document.protocolVersion).to.equal(protocolVersion.latestVersion);
    expect(identity.protocolVersion).to.equal(protocolVersion.latestVersion);

    dpp.setProtocolVersion(42);

    dataContract = dpp.dataContract.create(generateRandomIdentifier(), {
      niceDocument: {
        type: 'object',
        properties: {
          name: {
            type: 'string',
          },
        },
        required: ['$createdAt'],
        additionalProperties: false,
      },
    });
    identity = dpp.identity.create(getChainAssetLockProofFixture(), []);

    expect(dataContract.protocolVersion).to.equal(42);
    expect(identity.protocolVersion).to.equal(42);
  });
});
