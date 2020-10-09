const rewiremock = require('rewiremock/node');

const getDataContractFixture = require('../../../../lib/test/fixtures/getDataContractFixture');
const stateTransitionTypes = require('../../../../lib/stateTransition/stateTransitionTypes');

const DataContract = require('../../../../lib/dataContract/DataContract');

describe('DataContractCreateTransition', () => {
  let stateTransition;
  let dataContract;
  let hashMock;
  let encodeMock;

  beforeEach(function beforeEach() {
    hashMock = this.sinonSandbox.stub();
    const serializerMock = { encode: this.sinonSandbox.stub() };
    encodeMock = serializerMock.encode;

    const DataContractCreateTransition = rewiremock.proxy('../../../../lib/dataContract/stateTransition/DataContractCreateTransition', {
      '../../../../lib/util/hash': hashMock,
      '../../../../lib/util/serializer': serializerMock,
    });

    dataContract = getDataContractFixture();
    stateTransition = new DataContractCreateTransition({
      protocolVersion: DataContract.PROTOCOL_VERSION,
      dataContract: dataContract.toObject(),
      entropy: dataContract.getEntropy(),
    });
  });

  describe('#getProtocolVersion', () => {
    it('should return the current protocol version', () => {
      const result = stateTransition.getProtocolVersion();

      expect(result).to.equal(0);
    });
  });

  describe('#getType', () => {
    it('should return State Transition type', () => {
      const result = stateTransition.getType();

      expect(result).to.equal(stateTransitionTypes.DATA_CONTRACT_CREATE);
    });
  });

  describe('#getDataContract', () => {
    it('should return Data Contract', () => {
      const result = stateTransition.getDataContract();

      expect(result.toObject()).to.deep.equal(dataContract.toObject());
    });
  });

  describe('#toJSON', () => {
    it('should return State Transition as plain JS object', () => {
      expect(stateTransition.toJSON()).to.deep.equal({
        protocolVersion: DataContract.PROTOCOL_VERSION,
        type: stateTransitionTypes.DATA_CONTRACT_CREATE,
        dataContract: dataContract.toJSON(),
        signaturePublicKeyId: undefined,
        signature: undefined,
        entropy: dataContract.getEntropy().toString('base64'),
      });
    });
  });

  describe('#toBuffer', () => {
    it('should return serialized State Transition', () => {
      const serializedStateTransition = '123';

      encodeMock.returns(serializedStateTransition);

      const result = stateTransition.toBuffer();

      expect(result).to.equal(serializedStateTransition);

      expect(encodeMock.getCall(0).args).to.have.deep.members([
        stateTransition.toObject(),
      ]);
    });
  });

  describe('#hash', () => {
    it('should return State Transition hash as hex', () => {
      const serializedDocument = '123';
      const hashedDocument = '456';

      encodeMock.returns(serializedDocument);
      hashMock.returns(hashedDocument);

      const result = stateTransition.hash();

      expect(result).to.equal(hashedDocument);

      expect(encodeMock.getCall(0).args).to.have.deep.members([
        stateTransition.toObject(),
      ]);
      expect(hashMock).to.have.been.calledOnceWith(serializedDocument);
    });
  });

  describe('#getOwnerId', () => {
    it('should return owner id', async () => {
      const result = stateTransition.getOwnerId();

      expect(result).to.equal(stateTransition.getDataContract().getOwnerId());
    });
  });
});
