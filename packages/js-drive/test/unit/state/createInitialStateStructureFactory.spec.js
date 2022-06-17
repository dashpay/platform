const createInitialStateStructureFactory = require('../../../lib/state/createInitialStateStructureFactory');
const SpentAssetLockTransactionsRepository = require('../../../lib/identity/SpentAssetLockTransactionsRepository');

describe('createInitialStateStructureFactory', () => {
  let createInitialStateStructure;
  let identityRepositoryMock;
  let publicKeyToIdentitiesRepositoryMock;
  let groveDBStoreMock;
  let dataContractRepositoryMock;
  let spentAssetLockTransactionsRepositoryMock;

  beforeEach(function beforeEach() {
    identityRepositoryMock = {
      createTree: this.sinon.stub(),
    };

    publicKeyToIdentitiesRepositoryMock = {
      createTree: this.sinon.stub(),
    };

    dataContractRepositoryMock = {
      createTree: this.sinon.stub(),
    };

    groveDBStoreMock = {
      createTree: this.sinon.stub(),
    };

    spentAssetLockTransactionsRepositoryMock = {
      createTree: this.sinon.stub(),
    };

    createInitialStateStructure = createInitialStateStructureFactory(
      identityRepositoryMock,
      publicKeyToIdentitiesRepositoryMock,
      spentAssetLockTransactionsRepositoryMock,
      dataContractRepositoryMock,
      groveDBStoreMock,
    );
  });

  it('should create initial state structure', async () => {
    await createInitialStateStructure();

    expect(identityRepositoryMock.createTree)
      .to.be.calledOnceWithExactly({ useTransaction: true });
    expect(publicKeyToIdentitiesRepositoryMock.createTree)
      .to.be.calledOnceWithExactly({ useTransaction: true });
    expect(dataContractRepositoryMock.createTree)
      .to.be.calledOnceWithExactly({ useTransaction: true });

    expect(groveDBStoreMock.createTree)
      .to.be.calledOnceWithExactly(
        [],
        SpentAssetLockTransactionsRepository.TREE_PATH[0],
        { useTransaction: true },
      );

    expect(spentAssetLockTransactionsRepositoryMock.createTree)
      .to.be.calledOnceWithExactly({ useTransaction: true });
  });
});
