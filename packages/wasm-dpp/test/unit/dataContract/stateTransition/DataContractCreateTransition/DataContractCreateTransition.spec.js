const getDataContractFixture = require('../../../../../lib/test/fixtures/getDataContractFixture');
const { default: loadWasmDpp } = require('../../../../..');
const { getLatestProtocolVersion, StateTransitionTypes } = require('../../../../..');

describe('DataContractCreateTransition', () => {
  let stateTransition;
  let dataContract;
  let DataContractCreateTransition;
  let Identifier;

  before(async () => {
    ({
      DataContractCreateTransition, Identifier,
    } = await loadWasmDpp());
  });

  beforeEach(async () => {
    dataContract = await getDataContractFixture();

    stateTransition = new DataContractCreateTransition({
      protocolVersion: getLatestProtocolVersion(),
      dataContract: dataContract.toObject(),
      entropy: dataContract.getEntropy(),
    });
  });

  describe('#getProtocolVersion', () => {
    it('should return the current protocol version', () => {
      const result = stateTransition.getProtocolVersion();

      expect(result).to.equal(getLatestProtocolVersion());
    });
  });

  describe('#getType', () => {
    it('should return State Transition type', () => {
      const result = stateTransition.getType();

      expect(result).to.equal(StateTransitionTypes.DataContractCreate);
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
      const dc = dataContract.toJSON();
      delete dc.$defs;

      expect(stateTransition.toJSON(true)).to.deep.equal({
        protocolVersion: getLatestProtocolVersion(),
        type: StateTransitionTypes.DataContractCreate,
        dataContract: dc,
        entropy: dataContract.getEntropy().toString('base64'),
      });
    });
  });

  describe('#toBuffer', () => {
    it('should return serialized State Transition', () => {
      const result = stateTransition.toBuffer();
      expect(result).to.be.instanceOf(Buffer);
      expect(result).to.have.lengthOf(2271);
    });
  });

  describe('#getOwnerId', () => {
    it('should return owner id', async () => {
      const result = stateTransition.getOwnerId();
      const reference = stateTransition.getDataContract().getOwnerId();

      expect(result.toBuffer()).to.deep.equal(reference.toBuffer());
    });
  });

  describe('#getModifiedDataIds', () => {
    it('should return ids of affected data contracts', () => {
      const result = stateTransition.getModifiedDataIds();

      expect(result.length).to.be.equal(1);
      const contractId = result[0];

      expect(contractId).to.be.an.instanceOf(Identifier);
      expect(contractId.toBuffer()).to.be.deep.equal(dataContract.getId().toBuffer());
    });
  });

  describe('#isDataContractStateTransition', () => {
    it('should return true', () => {
      expect(stateTransition.isDataContractStateTransition()).to.be.true();
    });
  });

  describe('#isDocumentStateTransition', () => {
    it('should return false', () => {
      expect(stateTransition.isDocumentStateTransition()).to.be.false();
    });
  });

  describe('#isIdentityStateTransition', () => {
    it('should return false', () => {
      expect(stateTransition.isIdentityStateTransition()).to.be.false();
    });
  });
});
