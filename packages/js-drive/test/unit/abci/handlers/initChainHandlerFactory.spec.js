const Long = require('long');

const {
  tendermint: {
    abci: {
      ResponseInitChain,
      ValidatorSetUpdate,
    },
  },
} = require('@dashevo/abci/types');

const { PrivateKey } = require('@dashevo/dashcore-lib');

const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');
const initChainHandlerFactory = require('../../../../lib/abci/handlers/initChainHandlerFactory');
const LoggerMock = require('../../../../lib/test/mock/LoggerMock');

describe('initChainHandlerFactory', () => {
  let initChainHandler;
  let updateSimplifiedMasternodeListMock;
  let initialCoreChainLockedHeight;
  let validatorSetMock;
  let createValidatorSetUpdateMock;
  let loggerMock;
  let validatorSetUpdate;
  let registerSystemDataContractMock;
  let registerTopLevelDomainMock;
  let registerFeatureFlagMock;
  let rootTreeMock;
  let documentDatabaseManagerMock;
  let previousDocumentDatabaseManagerMock;
  let dpnsContractId;
  let dpnsOwnerId;
  let dpnsOwnerPublicKey;
  let dpnsDocuments;
  let featureFlagsContractId;
  let featureFlagsOwnerId;
  let featureFlagsOwnerPublicKey;
  let featureFlagsDocuments;
  let masternodeRewardSharesContractId;
  let masternodeRewardSharesOwnerId;
  let masternodeRewardSharesOwnerPublicKey;
  let masternodeRewardSharesDocuments;
  let dashpayContractId;
  let dashpayOwnerId;
  let dashpayOwnerPublicKey;
  let dashpayDocuments;
  let blockExecutionStoreTransactionsMock;
  let cloneToPreviousStoreTransactionsMock;
  let containerMock;

  beforeEach(function beforeEach() {
    initialCoreChainLockedHeight = 1;

    updateSimplifiedMasternodeListMock = this.sinon.stub();

    const quorumHash = Buffer.alloc(64).fill(1).toString('hex');
    validatorSetMock = {
      initialize: this.sinon.stub(),
      getQuorum: this.sinon.stub().returns({
        quorumHash,
      }),
    };

    validatorSetUpdate = new ValidatorSetUpdate();

    createValidatorSetUpdateMock = this.sinon.stub().returns(validatorSetUpdate);

    loggerMock = new LoggerMock(this.sinon);

    registerSystemDataContractMock = this.sinon.stub();
    registerTopLevelDomainMock = this.sinon.stub();
    registerFeatureFlagMock = this.sinon.stub();

    dpnsContractId = generateRandomIdentifier();
    dpnsOwnerId = generateRandomIdentifier();
    featureFlagsContractId = generateRandomIdentifier();
    featureFlagsOwnerId = generateRandomIdentifier();
    masternodeRewardSharesContractId = generateRandomIdentifier();
    masternodeRewardSharesOwnerId = generateRandomIdentifier();
    dashpayContractId = generateRandomIdentifier();
    dashpayOwnerId = generateRandomIdentifier();

    const privateKey = new PrivateKey(undefined, 'testnet');

    dpnsOwnerPublicKey = privateKey.toPublicKey();
    featureFlagsOwnerPublicKey = privateKey.toPublicKey();
    masternodeRewardSharesOwnerPublicKey = privateKey.toPublicKey();
    dashpayOwnerPublicKey = privateKey.toPublicKey();

    dpnsDocuments = { id: generateRandomIdentifier() };
    featureFlagsDocuments = { id: generateRandomIdentifier() };
    masternodeRewardSharesDocuments = { id: generateRandomIdentifier() };
    dashpayDocuments = { id: generateRandomIdentifier() };

    rootTreeMock = {
      getRootHash: this.sinon.stub(),
    };
    documentDatabaseManagerMock = {
      create: this.sinon.stub(),
    };
    previousDocumentDatabaseManagerMock = {
      create: this.sinon.stub(),
    };

    blockExecutionStoreTransactionsMock = {
      start: this.sinon.stub(),
      commit: this.sinon.stub(),
    };
    cloneToPreviousStoreTransactionsMock = this.sinon.stub();
    containerMock = {
      register: this.sinon.stub(),
    };

    initChainHandler = initChainHandlerFactory(
      updateSimplifiedMasternodeListMock,
      initialCoreChainLockedHeight,
      validatorSetMock,
      createValidatorSetUpdateMock,
      loggerMock,
      registerSystemDataContractMock,
      registerTopLevelDomainMock,
      registerFeatureFlagMock,
      rootTreeMock,
      documentDatabaseManagerMock,
      previousDocumentDatabaseManagerMock,
      dpnsContractId,
      dpnsOwnerId,
      dpnsOwnerPublicKey,
      dpnsDocuments,
      featureFlagsContractId,
      featureFlagsOwnerId,
      featureFlagsOwnerPublicKey,
      featureFlagsDocuments,
      masternodeRewardSharesContractId,
      masternodeRewardSharesOwnerId,
      masternodeRewardSharesOwnerPublicKey,
      masternodeRewardSharesDocuments,
      dashpayContractId,
      dashpayOwnerId,
      dashpayOwnerPublicKey,
      dashpayDocuments,
      blockExecutionStoreTransactionsMock,
      cloneToPreviousStoreTransactionsMock,
      containerMock,
    );
  });

  it('should update height, start transactions and return ResponseBeginBlock', async () => {
    const request = {
      initialHeight: Long.fromInt(1),
      chainId: 'test',
      time: {
        seconds: new Long((new Date()).getTime() / 1000),
      },
    };

    const response = await initChainHandler(request);

    expect(response).to.be.an.instanceOf(ResponseInitChain);
    expect(response.validatorSetUpdate).to.be.equal(validatorSetUpdate);
    expect(response.initialCoreHeight).to.be.equal(initialCoreChainLockedHeight);

    expect(updateSimplifiedMasternodeListMock).to.be.calledOnceWithExactly(
      initialCoreChainLockedHeight,
      {
        logger: loggerMock,
      },
    );

    expect(validatorSetMock.initialize).to.be.calledOnceWithExactly(
      initialCoreChainLockedHeight,
    );

    expect(createValidatorSetUpdateMock).to.be.calledOnceWithExactly(validatorSetMock);
  });
});
