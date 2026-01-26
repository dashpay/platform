import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('Group', () => {
  const memberIdHex = '1111111111111111111111111111111111111111111111111111111111111111';
  const member2IdHex = '2222222222222222222222222222222222222222222222222222222222222222';
  function createMembersMap(membersArray: Array<[InstanceType<typeof wasm.Identifier>, number]>) {
    const map = new Map<InstanceType<typeof wasm.Identifier>, number>();
    for (const [identifier, power] of membersArray) {
      map.set(identifier, power);
    }
    return map;
  }

  describe('constructor', () => {
    it('should create Group with members and required power', () => {
      const memberId = wasm.Identifier.fromBytes(Buffer.from(memberIdHex, 'hex'));
      const members = createMembersMap([[memberId, 100]]);

      const group = new wasm.Group(members, 50);

      expect(group.requiredPower).to.equal(50);
      expect(group.members).to.be.instanceOf(Map);
    });

    it('should create Group with multiple members', () => {
      const memberId1 = wasm.Identifier.fromBytes(Buffer.from(memberIdHex, 'hex'));
      const memberId2 = wasm.Identifier.fromBytes(Buffer.from(member2IdHex, 'hex'));
      const members = createMembersMap([
        [memberId1, 100],
        [memberId2, 50],
      ]);

      const group = new wasm.Group(members, 75);

      expect(group.requiredPower).to.equal(75);
    });
  });

  describe('getters and setters', () => {
    it('should get and set members', () => {
      const memberId = wasm.Identifier.fromBytes(Buffer.from(memberIdHex, 'hex'));
      const members = createMembersMap([[memberId, 100]]);

      const group = new wasm.Group(members, 50);

      const fetchedMembers = group.members;
      expect(fetchedMembers).to.be.instanceOf(Map);
      // Map keys are now base58 strings for value-based lookups
      const foundPower = fetchedMembers.get(memberId.toBase58());
      expect(foundPower).to.equal(100);
    });

    it('should get and set required power', () => {
      const memberId = wasm.Identifier.fromBytes(Buffer.from(memberIdHex, 'hex'));
      const members = createMembersMap([[memberId, 100]]);

      const group = new wasm.Group(members, 50);
      expect(group.requiredPower).to.equal(50);

      group.requiredPower = 75;
      expect(group.requiredPower).to.equal(75);
    });

    it('should set member required power', () => {
      const memberId = wasm.Identifier.fromBytes(Buffer.from(memberIdHex, 'hex'));
      const members = createMembersMap([[memberId, 100]]);

      const group = new wasm.Group(members, 50);

      group.setMemberRequiredPower(memberId, 200);

      const updatedMembers = group.members;
      // Map keys are now base58 strings for value-based lookups
      const foundPower = updatedMembers.get(memberId.toBase58());
      expect(foundPower).to.equal(200);
    });
  });

  describe('conversion methods', () => {
    it('should round-trip via toJSON/fromJSON', () => {
      const memberId = wasm.Identifier.fromBytes(Buffer.from(memberIdHex, 'hex'));
      const members = createMembersMap([[memberId, 100]]);

      const group = new wasm.Group(members, 50);

      const json = group.toJSON();
      expect(json).to.be.an('object');

      const restored = wasm.Group.fromJSON(json);
      expect(restored.requiredPower).to.equal(group.requiredPower);
    });

    it('should export toObject', () => {
      const memberId = wasm.Identifier.fromBytes(Buffer.from(memberIdHex, 'hex'));
      const members = createMembersMap([[memberId, 100]]);

      const group = new wasm.Group(members, 50);

      const obj = group.toObject();
      // toObject exports as Map type in serde_wasm_bindgen which doesn't round-trip
      // but it should at least be defined
      expect(obj).to.not.be.undefined();
    });

    it('should round-trip Group with multiple members via toJSON/fromJSON', () => {
      const memberId1 = wasm.Identifier.fromBytes(Buffer.from(memberIdHex, 'hex'));
      const memberId2 = wasm.Identifier.fromBytes(Buffer.from(member2IdHex, 'hex'));
      const members = createMembersMap([
        [memberId1, 100],
        [memberId2, 50],
      ]);

      const group = new wasm.Group(members, 75);

      const json = group.toJSON();
      const restored = wasm.Group.fromJSON(json);

      expect(restored.requiredPower).to.equal(75);
    });
  });

  describe('type properties', () => {
    it('should return correct __type', () => {
      const memberId = wasm.Identifier.fromBytes(Buffer.from(memberIdHex, 'hex'));
      const members = createMembersMap([[memberId, 100]]);

      const group = new wasm.Group(members, 50);

      expect(group.__type).to.equal('Group');
    });
  });
});
