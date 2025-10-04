import getWasm from './helpers/wasm.js';
import { value, dataContractsBytes } from './mocks/DataContract/index.js';
import { fromHexString } from './utils/hex.js';

let wasm;
let PlatformVersionWASM;

before(async () => {
  wasm = await getWasm();
  ({ PlatformVersionWASM } = wasm);
});

describe('DataContract Updatet Transition', () => {
  describe('serialization / deserialization', () => {
    it('should allow to create document_transitions from data contract', () => {
      const dataContract = wasm.DataContractWASM.fromValue(value, false, PlatformVersionWASM.PLATFORM_V1);

      const dataContractTransition = new wasm.DataContractUpdateTransitionWASM(dataContract, BigInt(1));

      expect(dataContractTransition.__wbg_ptr).to.not.equal(0);
      expect(dataContract.__wbg_ptr).to.not.equal(0);
    });

    it('should allow to convert document_transitions to bytes and create from bytes', () => {
      const dataContract = wasm.DataContractWASM.fromValue(value, false, PlatformVersionWASM.PLATFORM_V1);

      const dataContractTransition = new wasm.DataContractUpdateTransitionWASM(dataContract, BigInt(1));

      const bytes = dataContractTransition.bytes();

      const newDataContractTransition = wasm.DataContractUpdateTransitionWASM.fromBytes(bytes);

      expect(newDataContractTransition.bytes()).to.deep.equal(bytes);
      expect(newDataContractTransition.__wbg_ptr).to.not.equal(0);
      expect(dataContractTransition.__wbg_ptr).to.not.equal(0);
      expect(dataContract.__wbg_ptr).to.not.equal(0);
    });

    it('should allow to convert data contract transition to state document_transitions and create data contract transition from state transition', () => {
      const dataContract = wasm.DataContractWASM.fromValue(value, false, PlatformVersionWASM.PLATFORM_V1);

      const dataContractTransition = new wasm.DataContractUpdateTransitionWASM(dataContract, BigInt(1));

      const stateTransition = dataContractTransition.toStateTransition();

      const newDataContractTransition = wasm.DataContractUpdateTransitionWASM.fromStateTransition(stateTransition);

      expect(dataContractTransition.bytes()).to.deep.equal(newDataContractTransition.bytes());
    });
  });

  describe('getters', () => {
    it('should allow to get feature version', () => {
      const dataContract = wasm.DataContractWASM.fromValue(value, false, PlatformVersionWASM.PLATFORM_V1);

      const dataContractTransition = new wasm.DataContractUpdateTransitionWASM(dataContract, BigInt(1));

      expect(dataContractTransition.featureVersion).to.equal(0);
    });

    it('should allow to verify protocol version', () => {
      const dataContract = wasm.DataContractWASM.fromValue(value, false, PlatformVersionWASM.PLATFORM_V1);

      const dataContractTransition = new wasm.DataContractUpdateTransitionWASM(dataContract, BigInt(1));

      expect(dataContractTransition.verifyProtocolVersion(1)).to.equal(true);
    });

    it('should allow to verify incorrect protocol version', () => {
      const dataContract = wasm.DataContractWASM.fromValue(value, false, PlatformVersionWASM.PLATFORM_V1);

      const dataContractTransition = new wasm.DataContractUpdateTransitionWASM(dataContract, BigInt(1));

      try {
        dataContractTransition.verifyProtocolVersion(20);
        expect(true).to.equal(false);
      } catch (error) {
        expect(false).to.equal(false);
      }
    });

    it('should allow to get data contract', () => {
      const dataContract = wasm.DataContractWASM.fromValue(value, false, PlatformVersionWASM.PLATFORM_V1);

      const dataContractTransition = new wasm.DataContractUpdateTransitionWASM(dataContract, BigInt(1));

      const newDataContract = dataContractTransition.getDataContract();

      expect(dataContract.bytes()).to.deep.equal(newDataContract.bytes());
    });
  });

  describe('setters', () => {
    it('should allow to set the data contract', () => {
      const [dataContractBytes] = dataContractsBytes;

      const dataContract = wasm.DataContractWASM.fromValue(value, false, PlatformVersionWASM.PLATFORM_V1);

      const dataContractTransition = new wasm.DataContractUpdateTransitionWASM(dataContract, BigInt(1));

      const newDataContract = wasm.DataContractWASM.fromBytes(fromHexString(dataContractBytes), false, PlatformVersionWASM.PLATFORM_V1);

      dataContractTransition.setDataContract(newDataContract);

      expect(fromHexString(dataContractBytes)).to.deep.equal(newDataContract.bytes());
    });
  });
});
