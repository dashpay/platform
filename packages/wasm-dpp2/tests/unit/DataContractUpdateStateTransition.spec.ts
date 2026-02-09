import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';
import { value, dataContractsBytes } from './mocks/DataContract/index.js';
import { fromHexString } from './utils/hex.ts';

let PlatformVersion: typeof wasm.PlatformVersion;

before(async () => {
  await initWasm();
  ({ PlatformVersion } = wasm);
});

describe('DataContractUpdateTransition', () => {
  describe('constructor()', () => {
    it('should create transition from data contract', () => {
      const dataContract = wasm.DataContract.fromJSON(value, false, new PlatformVersion(1));

      const dataContractTransition = new wasm.DataContractUpdateTransition(dataContract, BigInt(1));

      expect(dataContractTransition).to.be.an.instanceof(wasm.DataContractUpdateTransition);
      expect(dataContract).to.be.an.instanceof(wasm.DataContract);
    });
  });

  describe('toBytes()', () => {
    it('should convert transition to bytes', () => {
      const dataContract = wasm.DataContract.fromJSON(value, false, new PlatformVersion(1));

      const dataContractTransition = new wasm.DataContractUpdateTransition(dataContract, BigInt(1));

      const bytes = dataContractTransition.toBytes();

      expect(bytes).to.be.an.instanceof(Uint8Array);
      expect(bytes.length).to.be.greaterThan(0);
    });
  });

  describe('fromBytes()', () => {
    it('should create transition from bytes', () => {
      const dataContract = wasm.DataContract.fromJSON(value, false, new PlatformVersion(1));

      const dataContractTransition = new wasm.DataContractUpdateTransition(dataContract, BigInt(1));

      const bytes = dataContractTransition.toBytes();

      const newDataContractTransition = wasm.DataContractUpdateTransition.fromBytes(bytes);

      expect(newDataContractTransition.toBytes()).to.deep.equal(bytes);
      expect(newDataContractTransition).to.be.an.instanceof(wasm.DataContractUpdateTransition);
      expect(dataContractTransition).to.be.an.instanceof(wasm.DataContractUpdateTransition);
      expect(dataContract).to.be.an.instanceof(wasm.DataContract);
    });
  });

  describe('toStateTransition()', () => {
    it('should convert to state transition', () => {
      const dataContract = wasm.DataContract.fromJSON(value, false, new PlatformVersion(1));

      const dataContractTransition = new wasm.DataContractUpdateTransition(dataContract, BigInt(1));

      const stateTransition = dataContractTransition.toStateTransition();

      const newDataContractTransition = wasm.DataContractUpdateTransition.fromStateTransition(stateTransition);

      expect(dataContractTransition.toBytes()).to.deep.equal(newDataContractTransition.toBytes());
    });
  });

  describe('fromStateTransition()', () => {
    it('should create transition from state transition', () => {
      const dataContract = wasm.DataContract.fromJSON(value, false, new PlatformVersion(1));

      const dataContractTransition = new wasm.DataContractUpdateTransition(dataContract, BigInt(1));

      const stateTransition = dataContractTransition.toStateTransition();

      const newDataContractTransition = wasm.DataContractUpdateTransition.fromStateTransition(stateTransition);

      expect(dataContractTransition.toBytes()).to.deep.equal(newDataContractTransition.toBytes());
    });
  });

  describe('featureVersion', () => {
    it('should return feature version', () => {
      const dataContract = wasm.DataContract.fromJSON(value, false, new PlatformVersion(1));

      const dataContractTransition = new wasm.DataContractUpdateTransition(dataContract, BigInt(1));

      expect(dataContractTransition.featureVersion).to.equal(0);
    });
  });

  describe('verifyProtocolVersion()', () => {
    it('should return true for valid protocol version', () => {
      const dataContract = wasm.DataContract.fromJSON(value, false, new PlatformVersion(1));

      const dataContractTransition = new wasm.DataContractUpdateTransition(dataContract, BigInt(1));

      expect(dataContractTransition.verifyProtocolVersion(1)).to.equal(true);
    });

    it('should throw for invalid protocol version', () => {
      const dataContract = wasm.DataContract.fromJSON(value, false, new PlatformVersion(1));

      const dataContractTransition = new wasm.DataContractUpdateTransition(dataContract, BigInt(1));

      try {
        dataContractTransition.verifyProtocolVersion(20);
        expect(true).to.equal(false);
      } catch {
        expect(false).to.equal(false);
      }
    });
  });

  describe('getDataContract()', () => {
    it('should return data contract', () => {
      const dataContract = wasm.DataContract.fromJSON(value, false, new PlatformVersion(1));

      const dataContractTransition = new wasm.DataContractUpdateTransition(dataContract, BigInt(1));

      const newDataContract = dataContractTransition.getDataContract();

      expect(dataContract.toBytes()).to.deep.equal(newDataContract.toBytes());
    });
  });

  describe('setDataContract()', () => {
    it('should set the data contract', () => {
      const [dataContractBytes] = dataContractsBytes;

      const dataContract = wasm.DataContract.fromJSON(value, false, new PlatformVersion(1));

      const dataContractTransition = new wasm.DataContractUpdateTransition(dataContract, BigInt(1));

      const newDataContract = wasm.DataContract.fromBytes(fromHexString(dataContractBytes), false, new PlatformVersion(1));

      dataContractTransition.setDataContract(newDataContract);

      expect(fromHexString(dataContractBytes)).to.deep.equal(newDataContract.toBytes(new PlatformVersion(1)));
    });
  });
});
