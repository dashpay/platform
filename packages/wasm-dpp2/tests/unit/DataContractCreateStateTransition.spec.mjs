import getWasm from './helpers/wasm.js';
import { value, dataContractsBytes } from './mocks/DataContract/index.js';
import { fromHexString } from './utils/hex.js';

let wasm;
let PlatformVersion;

before(async () => {
  wasm = await getWasm();
  ({ PlatformVersion } = wasm);
});

describe('DataContract Create Transition', () => {
  describe('serialization / deserialization', () => {
    it('should allow to create document_transitions from data contract', () => {
      const dataContract = wasm.DataContract.fromValue(value, false, PlatformVersion.PLATFORM_V1);

      const dataContractTransition = new wasm.DataContractCreateTransition(dataContract, BigInt(1));

      expect(dataContractTransition.__wbg_ptr).to.not.equal(0);
      expect(dataContract.__wbg_ptr).to.not.equal(0);
    });

    it('should allow to convert document_transitions to bytes and create from bytes', () => {
      const dataContract = wasm.DataContract.fromValue(value, false, PlatformVersion.PLATFORM_V1);

      const dataContractTransition = new wasm.DataContractCreateTransition(dataContract, BigInt(1));

      const bytes = dataContractTransition.toBytes();

      const newDataContractTransition = wasm.DataContractCreateTransition.fromBytes(bytes);

      expect(newDataContractTransition.toBytes()).to.deep.equal(bytes);
      expect(newDataContractTransition.__wbg_ptr).to.not.equal(0);
      expect(dataContractTransition.__wbg_ptr).to.not.equal(0);
      expect(dataContract.__wbg_ptr).to.not.equal(0);
    });

    it('should allow to convert data contract transition to state document_transitions and create data contract transition from state transition', () => {
      const dataContract = wasm.DataContract.fromValue(value, false, PlatformVersion.PLATFORM_V1);

      const dataContractTransition = new wasm.DataContractCreateTransition(dataContract, BigInt(1));

      const stateTransition = dataContractTransition.toStateTransition();

      const newDataContractTransition = wasm.DataContractCreateTransition.fromStateTransition(stateTransition);

      expect(dataContractTransition.toBytes()).to.deep.equal(newDataContractTransition.toBytes());
    });
  });

  describe('getters', () => {
    it('should allow to get feature version', () => {
      const dataContract = wasm.DataContract.fromValue(value, false, PlatformVersion.PLATFORM_V1);

      const dataContractTransition = new wasm.DataContractCreateTransition(dataContract, BigInt(1));

      expect(dataContractTransition.featureVersion).to.equal(0);
    });

    it('should allow to verify protocol version', () => {
      const dataContract = wasm.DataContract.fromValue(value, false, PlatformVersion.PLATFORM_V1);

      const dataContractTransition = new wasm.DataContractCreateTransition(dataContract, BigInt(1));

      expect(dataContractTransition.verifyProtocolVersion(1)).to.equal(true);
    });

    it('should allow to verify incorrect protocol version', () => {
      const dataContract = wasm.DataContract.fromValue(value, false, PlatformVersion.PLATFORM_V1);

      const dataContractTransition = new wasm.DataContractCreateTransition(dataContract, BigInt(1));

      try {
        dataContractTransition.verifyProtocolVersion(20);
        expect(true).to.equal(false);
      } catch (error) {
        expect(false).to.equal(false);
      }
    });

    it('should allow to get data contract', () => {
      const dataContract = wasm.DataContract.fromValue(value, false, PlatformVersion.PLATFORM_V1);

      const dataContractTransition = new wasm.DataContractCreateTransition(dataContract, BigInt(1));

      const newDataContract = dataContractTransition.getDataContract();

      expect(dataContract.toBytes()).to.deep.equal(newDataContract.toBytes());
    });
  });

  describe('setters', () => {
    it('should allow to set the data contract', () => {
      const [dataContractBytes] = dataContractsBytes;

      const dataContract = wasm.DataContract.fromValue(value, false, PlatformVersion.PLATFORM_V1);

      const dataContractTransition = new wasm.DataContractCreateTransition(dataContract, BigInt(1));

      const newDataContract = wasm.DataContract.fromBytes(fromHexString(dataContractBytes), false, PlatformVersion.PLATFORM_V1);

      dataContractTransition.setDataContract(newDataContract);

      expect(fromHexString(dataContractBytes)).to.deep.equal(dataContractTransition.getDataContract().toBytes());
    });
  });
});
