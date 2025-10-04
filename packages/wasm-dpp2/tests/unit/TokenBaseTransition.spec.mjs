import getWasm from './helpers/wasm.js';
import { dataContractId, ownerId } from './mocks/Document/index.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

describe('TokenBaseTransition', function () {
  describe('serialization / deserialization', function () {
    it('should allow to create from values', () => {
      const baseTransition = new wasm.TokenBaseTransitionWASM(BigInt(1), 1, dataContractId, ownerId)

      expect(baseTransition.__wbg_ptr).to.not.equal(0)
    })
  })

  describe('getters', function () {
    it('should allow to get identityContractNonce', () => {
      const baseTransition = new wasm.TokenBaseTransitionWASM(BigInt(1), 1, dataContractId, ownerId)

      expect(baseTransition.identityContractNonce).to.deep.equal(1n)
    })

    it('should allow to get tokenContractPosition', () => {
      const baseTransition = new wasm.TokenBaseTransitionWASM(BigInt(1), 1, dataContractId, ownerId)

      expect(baseTransition.tokenContractPosition).to.deep.equal(1)
    })

    it('should allow to get dataContractId', () => {
      const baseTransition = new wasm.TokenBaseTransitionWASM(BigInt(1), 1, dataContractId, ownerId)

      expect(baseTransition.dataContractId.base58()).to.deep.equal(dataContractId)
    })

    it('should allow to get tokenId', () => {
      const baseTransition = new wasm.TokenBaseTransitionWASM(BigInt(1), 1, dataContractId, ownerId)

      expect(baseTransition.tokenId.base58()).to.deep.equal(ownerId)
    })

    it('should allow to get usingGroupInfo', () => {
      const groupStInfo = new wasm.GroupStateTransitionInfoWASM(2, dataContractId, false)

      const baseTransition = new wasm.TokenBaseTransitionWASM(BigInt(1), 1, dataContractId, ownerId, groupStInfo)

      expect(groupStInfo.__wbg_ptr).to.not.equal(0)
      expect(baseTransition.usingGroupInfo.constructor.name).to.deep.equal('GroupStateTransitionInfoWASM')
    })
  })

  describe('setters', function () {
    it('should allow to set identityContractNonce', () => {
      const baseTransition = new wasm.TokenBaseTransitionWASM(BigInt(1), 1, dataContractId, ownerId)

      baseTransition.identityContractNonce = 3n

      expect(baseTransition.identityContractNonce).to.deep.equal(3n)
    })

    it('should allow to set tokenContractPosition', () => {
      const baseTransition = new wasm.TokenBaseTransitionWASM(BigInt(1), 1, dataContractId, ownerId)

      baseTransition.tokenContractPosition = 3

      expect(baseTransition.tokenContractPosition).to.deep.equal(3)
    })

    it('should allow to set dataContractId', () => {
      const baseTransition = new wasm.TokenBaseTransitionWASM(BigInt(1), 1, dataContractId, ownerId)

      baseTransition.dataContractId = ownerId

      expect(baseTransition.dataContractId.base58()).to.deep.equal(ownerId)
    })

    it('should allow to set tokenId', () => {
      const baseTransition = new wasm.TokenBaseTransitionWASM(BigInt(1), 1, dataContractId, ownerId)

      baseTransition.tokenId = dataContractId

      expect(baseTransition.tokenId.base58()).to.deep.equal(dataContractId)
    })

    it('should allow to set usingGroupInfo', () => {
      const groupStInfo = new wasm.GroupStateTransitionInfoWASM(2, dataContractId, false)

      const baseTransition = new wasm.TokenBaseTransitionWASM(BigInt(1), 1, dataContractId, ownerId)

      expect(baseTransition.usingGroupInfo).to.deep.equal(undefined)

      baseTransition.usingGroupInfo = groupStInfo

      expect(groupStInfo.__wbg_ptr).to.not.equal(0)
      expect(baseTransition.usingGroupInfo.constructor.name).to.deep.equal('GroupStateTransitionInfoWASM')
    })
  })
})
