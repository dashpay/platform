import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';
import { dataContractId, ownerId } from './mocks/Document/index.js';

before(async () => {
  await initWasm();
});

interface BaseTransitionOptions {
  identityContractNonce?: bigint;
  tokenContractPosition?: number;
  dataContractId?: string;
  tokenId?: string;
  usingGroupInfo?: unknown;
}

describe('TokenBaseTransition', () => {
  function createBaseTransition(options: BaseTransitionOptions = {}) {
    return new wasm.TokenBaseTransition({
      identityContractNonce: options.identityContractNonce ?? BigInt(1),
      tokenContractPosition: options.tokenContractPosition ?? 1,
      dataContractId: options.dataContractId ?? dataContractId,
      tokenId: options.tokenId ?? ownerId,
      usingGroupInfo: options.usingGroupInfo,
    });
  }

  describe('constructor', () => {
    it('should create instance from values', () => {
      const baseTransition = createBaseTransition();

      expect(baseTransition).to.be.an.instanceof(wasm.TokenBaseTransition);
    });
  });

  describe('identityContractNonce', () => {
    it('should return identityContractNonce', () => {
      const baseTransition = createBaseTransition();

      expect(baseTransition.identityContractNonce).to.deep.equal(1n);
    });

    it('should set identityContractNonce', () => {
      const baseTransition = createBaseTransition();

      baseTransition.identityContractNonce = 3n;

      expect(baseTransition.identityContractNonce).to.deep.equal(3n);
    });
  });

  describe('tokenContractPosition', () => {
    it('should return tokenContractPosition', () => {
      const baseTransition = createBaseTransition();

      expect(baseTransition.tokenContractPosition).to.deep.equal(1);
    });

    it('should set tokenContractPosition', () => {
      const baseTransition = createBaseTransition();

      baseTransition.tokenContractPosition = 3;

      expect(baseTransition.tokenContractPosition).to.deep.equal(3);
    });
  });

  describe('dataContractId', () => {
    it('should return dataContractId', () => {
      const baseTransition = createBaseTransition();

      expect(baseTransition.dataContractId.toBase58()).to.deep.equal(dataContractId);
    });

    it('should set dataContractId', () => {
      const baseTransition = createBaseTransition();

      baseTransition.dataContractId = ownerId;

      expect(baseTransition.dataContractId.toBase58()).to.deep.equal(ownerId);
    });
  });

  describe('tokenId', () => {
    it('should return tokenId', () => {
      const baseTransition = createBaseTransition();

      expect(baseTransition.tokenId.toBase58()).to.deep.equal(ownerId);
    });

    it('should set tokenId', () => {
      const baseTransition = createBaseTransition();

      baseTransition.tokenId = dataContractId;

      expect(baseTransition.tokenId.toBase58()).to.deep.equal(dataContractId);
    });
  });

  describe('usingGroupInfo', () => {
    it('should return usingGroupInfo', () => {
      const groupStInfo = new wasm.GroupStateTransitionInfo({
        groupContractPosition: 2,
        actionId: dataContractId,
        isActionProposer: false,
      });

      const baseTransition = createBaseTransition({ usingGroupInfo: groupStInfo });

      expect(groupStInfo).to.be.an.instanceof(wasm.GroupStateTransitionInfo);
      expect(baseTransition.usingGroupInfo.constructor.name).to.deep.equal('GroupStateTransitionInfo');
    });

    it('should set usingGroupInfo', () => {
      const groupStInfo = new wasm.GroupStateTransitionInfo({
        groupContractPosition: 2,
        actionId: dataContractId,
        isActionProposer: false,
      });

      const baseTransition = createBaseTransition();

      expect(baseTransition.usingGroupInfo).to.deep.equal(undefined);

      baseTransition.usingGroupInfo = groupStInfo;

      expect(groupStInfo).to.be.an.instanceof(wasm.GroupStateTransitionInfo);
      expect(baseTransition.usingGroupInfo.constructor.name).to.deep.equal('GroupStateTransitionInfo');
    });
  });
});
