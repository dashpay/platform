import getWasm from './helpers/wasm.js';
import { dataContractId, ownerId } from './mocks/Document/index.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

describe('TokenBaseTransition', function () {
  function createBaseTransition(options = {}) {
    return new wasm.TokenBaseTransition({
      identityContractNonce: options.identityContractNonce ?? BigInt(1),
      tokenContractPosition: options.tokenContractPosition ?? 1,
      dataContractId: options.dataContractId ?? dataContractId,
      tokenId: options.tokenId ?? ownerId,
      usingGroupInfo: options.usingGroupInfo,
    });
  }

  describe('serialization / deserialization', function () {
    it('should allow to create from values', () => {
      const baseTransition = createBaseTransition();

      expect(baseTransition.__wbg_ptr).to.not.equal(0)
    })
  })

  describe('getters', function () {
    it('should allow to get identityContractNonce', () => {
      const baseTransition = createBaseTransition();

      expect(baseTransition.identityContractNonce).to.deep.equal(1n)
    })

    it('should allow to get tokenContractPosition', () => {
      const baseTransition = createBaseTransition();

      expect(baseTransition.tokenContractPosition).to.deep.equal(1)
    })

    it('should allow to get dataContractId', () => {
      const baseTransition = createBaseTransition();

      expect(baseTransition.dataContractId.toBase58()).to.deep.equal(dataContractId)
    })

    it('should allow to get tokenId', () => {
      const baseTransition = createBaseTransition();

      expect(baseTransition.tokenId.toBase58()).to.deep.equal(ownerId)
    })

    it('should allow to get usingGroupInfo', () => {
      const groupStInfo = new wasm.GroupStateTransitionInfo(2, dataContractId, false)

      const baseTransition = createBaseTransition({ usingGroupInfo: groupStInfo });

      expect(groupStInfo.__wbg_ptr).to.not.equal(0)
      expect(baseTransition.usingGroupInfo.constructor.name).to.deep.equal('GroupStateTransitionInfo')
    })
  })

  describe('setters', function () {
    it('should allow to set identityContractNonce', () => {
      const baseTransition = createBaseTransition();

      baseTransition.identityContractNonce = 3n

      expect(baseTransition.identityContractNonce).to.deep.equal(3n)
    })

    it('should allow to set tokenContractPosition', () => {
      const baseTransition = createBaseTransition();

      baseTransition.tokenContractPosition = 3

      expect(baseTransition.tokenContractPosition).to.deep.equal(3)
    })

    it('should allow to set dataContractId', () => {
      const baseTransition = createBaseTransition();

      baseTransition.dataContractId = ownerId

      expect(baseTransition.dataContractId.toBase58()).to.deep.equal(ownerId)
    })

    it('should allow to set tokenId', () => {
      const baseTransition = createBaseTransition();

      baseTransition.tokenId = dataContractId

      expect(baseTransition.tokenId.toBase58()).to.deep.equal(dataContractId)
    })

    it('should allow to set usingGroupInfo', () => {
      const groupStInfo = new wasm.GroupStateTransitionInfo(2, dataContractId, false)

      const baseTransition = createBaseTransition();

      expect(baseTransition.usingGroupInfo).to.deep.equal(undefined)

      baseTransition.usingGroupInfo = groupStInfo

      expect(groupStInfo.__wbg_ptr).to.not.equal(0)
      expect(baseTransition.usingGroupInfo.constructor.name).to.deep.equal('GroupStateTransitionInfo')
    })
  })
})
