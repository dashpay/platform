import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('Group', () => {
  const memberIdHex = '1111111111111111111111111111111111111111111111111111111111111111';
  const member2IdHex = '2222222222222222222222222222222222222222222222222222222222222222';
  function createMembersMap(membersArray: Array<[InstanceType<typeof wasm.Identifier>, number]>) {
    const map = new Map<string, number>();
    for (const [identifier, power] of membersArray) {
      map.set(identifier.toBase58(), power);
    }
    return map;
  }

  describe('constructor()', () => {
    it('should create Group with members and required power', () => {
      const memberId = wasm.Identifier.fromHex(memberIdHex);
      const members = createMembersMap([[memberId, 100]]);

      const group = new wasm.Group(members, 50);

      expect(group.requiredPower).to.equal(50);
      expect(group.members).to.be.instanceOf(Map);
    });

    it('should create Group with multiple members', () => {
      const memberId1 = wasm.Identifier.fromHex(memberIdHex);
      const memberId2 = wasm.Identifier.fromHex(member2IdHex);
      const members = createMembersMap([
        [memberId1, 100],
        [memberId2, 50],
      ]);

      const group = new wasm.Group(members, 75);

      expect(group.requiredPower).to.equal(75);
    });
  });

  describe('members', () => {
    it('should return members', () => {
      const memberId = wasm.Identifier.fromHex(memberIdHex);
      const members = createMembersMap([[memberId, 100]]);

      const group = new wasm.Group(members, 50);

      const fetchedMembers = group.members;
      expect(fetchedMembers).to.be.instanceOf(Map);
      // Map keys are now base58 strings for value-based lookups
      const foundPower = fetchedMembers.get(memberId.toBase58());
      expect(foundPower).to.equal(100);
    });
  });

  describe('requiredPower', () => {
    it('should return required power', () => {
      const memberId = wasm.Identifier.fromHex(memberIdHex);
      const members = createMembersMap([[memberId, 100]]);

      const group = new wasm.Group(members, 50);
      expect(group.requiredPower).to.equal(50);
    });

    it('should set required power', () => {
      const memberId = wasm.Identifier.fromHex(memberIdHex);
      const members = createMembersMap([[memberId, 100]]);

      const group = new wasm.Group(members, 50);

      group.requiredPower = 75;
      expect(group.requiredPower).to.equal(75);
    });
  });

  describe('setMemberRequiredPower()', () => {
    it('should set member required power', () => {
      const memberId = wasm.Identifier.fromHex(memberIdHex);
      const members = createMembersMap([[memberId, 100]]);

      const group = new wasm.Group(members, 50);

      group.setMemberRequiredPower(memberId, 200);

      const updatedMembers = group.members;
      // Map keys are now base58 strings for value-based lookups
      const foundPower = updatedMembers.get(memberId.toBase58());
      expect(foundPower).to.equal(200);
    });
  });

  describe('toJSON()', () => {
    it('should convert to JSON matching fixture', () => {
      const memberId = wasm.Identifier.fromHex(memberIdHex);
      const memberIdBase58 = memberId.toBase58();
      const members = createMembersMap([[memberId, 100]]);

      const group = new wasm.Group(members, 50);

      const json = group.toJSON();
      expect(json).to.deep.equal({
        $formatVersion: '0',
        members: {
          [memberIdBase58]: 100,
        },
        requiredPower: 50,
      });
    });

    it('should convert to JSON with multiple members', () => {
      const memberId1 = wasm.Identifier.fromHex(memberIdHex);
      const memberId2 = wasm.Identifier.fromHex(member2IdHex);
      const members = createMembersMap([
        [memberId1, 100],
        [memberId2, 50],
      ]);

      const group = new wasm.Group(members, 75);

      const json = group.toJSON();
      expect(json.$formatVersion).to.equal('0');
      expect(json.requiredPower).to.equal(75);
      expect(json.members[memberId1.toBase58()]).to.equal(100);
      expect(json.members[memberId2.toBase58()]).to.equal(50);
    });
  });

  describe('fromJSON()', () => {
    it('should create from JSON fixture and verify getters', () => {
      const memberId = wasm.Identifier.fromHex(memberIdHex);
      const memberIdBase58 = memberId.toBase58();

      const fixture = {
        $formatVersion: '0',
        members: {
          [memberIdBase58]: 100,
        },
        requiredPower: 50,
      };

      const restored = wasm.Group.fromJSON(fixture);

      expect(restored.requiredPower).to.equal(50);
      const restoredMembers = restored.members;
      expect(restoredMembers).to.be.instanceOf(Map);
      expect(restoredMembers.get(memberIdBase58)).to.equal(100);
    });

    it('should roundtrip Group with multiple members via toJSON/fromJSON', () => {
      const memberId1 = wasm.Identifier.fromHex(memberIdHex);
      const memberId2 = wasm.Identifier.fromHex(member2IdHex);
      const members = createMembersMap([
        [memberId1, 100],
        [memberId2, 50],
      ]);

      const group = new wasm.Group(members, 75);

      const json = group.toJSON();
      const restored = wasm.Group.fromJSON(json);

      expect(restored.requiredPower).to.equal(75);
      expect(restored.members.get(memberId1.toBase58())).to.equal(100);
      expect(restored.members.get(memberId2.toBase58())).to.equal(50);
    });
  });

  describe('toObject()', () => {
    it('should convert to Object matching fixture', () => {
      const memberId = wasm.Identifier.fromHex(memberIdHex);
      const memberIdBase58 = memberId.toBase58();
      const members = createMembersMap([[memberId, 100]]);

      const group = new wasm.Group(members, 50);

      // toObject uses toJSON internally for Group (due to BTreeMap<Identifier, u32>)
      const obj = group.toObject();
      expect(obj).to.deep.equal({
        $formatVersion: '0',
        members: {
          [memberIdBase58]: 100,
        },
        requiredPower: 50,
      });
    });
  });

  describe('fromObject()', () => {
    it('should create from Object fixture and verify getters', () => {
      const memberId = wasm.Identifier.fromHex(memberIdHex);
      const memberIdBase58 = memberId.toBase58();

      const fixture = {
        $formatVersion: '0',
        members: {
          [memberIdBase58]: 100,
        },
        requiredPower: 50,
      };

      const restored = wasm.Group.fromObject(fixture);

      expect(restored.requiredPower).to.equal(50);
      const restoredMembers = restored.members;
      expect(restoredMembers).to.be.instanceOf(Map);
      expect(restoredMembers.get(memberIdBase58)).to.equal(100);
    });
  });

  describe('__type', () => {
    it('should return correct __type', () => {
      const memberId = wasm.Identifier.fromHex(memberIdHex);
      const members = createMembersMap([[memberId, 100]]);

      const group = new wasm.Group(members, 50);

      expect(group.__type).to.equal('Group');
    });
  });
});
