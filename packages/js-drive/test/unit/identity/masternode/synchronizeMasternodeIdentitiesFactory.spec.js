const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');
const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');

const synchronizeMasternodeIdentitiesFactory = require('../../../../lib/identity/masternode/synchronizeMasternodeIdentitiesFactory');

describe('synchronizeMasternodeIdentitiesFactory', () => {
  let synchronizeMasternodeIdentities;
  let transactionalDppMock;
  let stateRepositoryMock;
  let createMasternodeIdentityMock;
  let simplifiedMasternodeListMock;
  let dataContractRepositoryMock;
  let smlStore;
  let coreHeight;

  beforeEach(function beforeEach() {
    transactionalDppMock = createDPPMock(this.sinon);
    stateRepositoryMock = createStateRepositoryMock(this.sinon);
    createMasternodeIdentityMock = this.sinon.stub();
    simplifiedMasternodeListMock = {
      getStore: this.sinon.stub(),
    };
    dataContractRepositoryMock = {
      fetch: this.sinon.stub(),
    };

    smlStore = {};

    simplifiedMasternodeListMock.getStore.returns(smlStore);

    synchronizeMasternodeIdentities = synchronizeMasternodeIdentitiesFactory(
      transactionalDppMock,
      stateRepositoryMock,
      createMasternodeIdentityMock,
      simplifiedMasternodeListMock,
      dataContractRepositoryMock,
    );

    coreHeight = 3;
  });

  it('should ___ if lastSyncedCoreHeight = 0', async () => {
    await synchronizeMasternodeIdentities(coreHeight);

    expect.fail('implement me');
  });

  it('should ___ if lastSyncedCoreHeight != 0', async () => {
    await synchronizeMasternodeIdentities(coreHeight);
    expect.fail('implement me');
  });

  it('should split documents into chunks', async () => {
    await synchronizeMasternodeIdentities(coreHeight);
    expect.fail('implement me');
  });
});
