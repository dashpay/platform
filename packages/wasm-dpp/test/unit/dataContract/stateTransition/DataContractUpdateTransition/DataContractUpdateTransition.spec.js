const getDataContractFixture = require('../../../../../lib/test/fixtures/getDataContractFixture');
const { default: loadWasmDpp } = require('../../../../..');
const { StateTransitionTypes } = require('../../../../..');

describe('DataContractUpdateTransition', () => {
  let stateTransition;
  let dataContract;
  let DataContractUpdateTransition;
  let Identifier;

  before(async () => {
    ({
      DataContractUpdateTransition, Identifier,
    } = await loadWasmDpp());
  });

  beforeEach(async () => {
    dataContract = await getDataContractFixture();

    stateTransition = new DataContractUpdateTransition({
      protocolVersion: 1,
      dataContract: dataContract.toObject(),
      identityContractNonce: 1,
    });
  });

  describe.skip('#getProtocolVersion', () => {
    it('should return the current protocol version', () => {
      const result = stateTransition.getProtocolVersion();

      expect(result).to.equal(1);
    });
  });

  describe('#getType', () => {
    it('should return State Transition type', () => {
      const result = stateTransition.getType();

      expect(result).to.equal(StateTransitionTypes.DataContractUpdate);
    });
  });

  describe('#getDataContract', () => {
    it('should return Data Contract', () => {
      const result = stateTransition.getDataContract();

      expect(result.toObject()).to.deep.equal(dataContract.toObject());
    });
  });

  describe.skip('#toJSON', () => {
    it('should return State Transition as plain JS object', () => {
      const dc = dataContract.toJSON();
      delete dc.$defs;

      expect(stateTransition.toJSON(true)).to.deep.equal({
        protocolVersion: 1,
        type: StateTransitionTypes.DataContractUpdate,
        dataContract: dc,
      });
    });
  });

  describe('#toBuffer', () => {
    it('should return serialized State Transition', () => {
      const result = stateTransition.toBuffer();
      expect(result).to.be.instanceOf(Buffer);
      expect(result).to.have.lengthOf(2359);
    });

    it('should be able to restore contract config from bytes', () => {
      const config = stateTransition.getDataContract().getConfig();
      config.keepsHistory = true;
      expect(stateTransition.getDataContract().getConfig().keepsHistory).to.be.true();
      const buffer = stateTransition.toBuffer();
      const restoredSt = DataContractUpdateTransition.fromBuffer(buffer);
      expect(restoredSt.getDataContract().getConfig().keepsHistory).to.be.true();
    });
  });

  describe.skip('#hash', () => {
    it('should return State Transition hash as hex', () => {
      // const jsStateTransition = new JsDataContractUpdateTransition(stateTransition.toJSON());
      //
      // const result = stateTransition.hash();
      // const resultJs = jsStateTransition.hash();
      //
      // expect(result).to.equal(resultJs);
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
