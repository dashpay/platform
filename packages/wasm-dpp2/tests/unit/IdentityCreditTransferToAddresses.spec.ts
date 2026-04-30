import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('IdentityCreditTransferToAddresses', () => {
  const addr1Bytes = new Uint8Array([
    0x00, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1,
  ]);

  function createTransition() {
    const outputAddr = wasm.PlatformAddress.fromBytes(addr1Bytes);
    const output = new wasm.PlatformAddressOutput(outputAddr, BigInt(90000));

    return new wasm.IdentityCreditTransferToAddresses({
      recipientAddresses: [output],
      senderId: '11111111111111111111111111111111',
      nonce: BigInt(1),
    });
  }

  describe('constructor()', () => {
    it('should create transition with string senderId', () => {
      const transition = createTransition();
      expect(transition).to.exist();
      expect(transition).to.be.an.instanceof(wasm.IdentityCreditTransferToAddresses);
    });

    it('should create transition with Identifier senderId', () => {
      const sender = new wasm.Identifier('11111111111111111111111111111111');
      const outputAddr = wasm.PlatformAddress.fromBytes(addr1Bytes);
      const output = new wasm.PlatformAddressOutput(outputAddr, BigInt(90000));

      const transition = new wasm.IdentityCreditTransferToAddresses({
        recipientAddresses: [output],
        senderId: sender,
        nonce: BigInt(1),
      });

      expect(transition).to.exist();
    });

    it('should create transition with user fee increase', () => {
      const outputAddr = wasm.PlatformAddress.fromBytes(addr1Bytes);
      const output = new wasm.PlatformAddressOutput(outputAddr, BigInt(90000));

      const transition = new wasm.IdentityCreditTransferToAddresses({
        recipientAddresses: [output],
        senderId: '11111111111111111111111111111111',
        nonce: BigInt(1),
        userFeeIncrease: 50,
      });

      expect(transition.userFeeIncrease).to.equal(50);
    });
  });

  describe('toBytes() / fromBytes()', () => {
    it('should round-trip via bytes', () => {
      const transition = createTransition();
      const bytes = transition.toBytes();
      const restored = wasm.IdentityCreditTransferToAddresses.fromBytes(bytes);
      expect(Buffer.from(restored.toBytes())).to.deep.equal(Buffer.from(bytes));
    });
  });

  describe('toBase64() / fromBase64()', () => {
    it('should round-trip via base64', () => {
      const transition = createTransition();
      const base64 = transition.toBase64();
      const bytes = transition.toBytes();
      expect(Buffer.from(base64, 'base64')).to.deep.equal(Buffer.from(bytes));

      const restored = wasm.IdentityCreditTransferToAddresses.fromBase64(base64);
      expect(Buffer.from(restored.toBytes())).to.deep.equal(Buffer.from(bytes));
    });
  });

  describe('toHex() / fromHex()', () => {
    it('should round-trip via hex', () => {
      const transition = createTransition();
      const hex = transition.toHex();
      const bytes = transition.toBytes();

      const restored = wasm.IdentityCreditTransferToAddresses.fromHex(hex);
      expect(Buffer.from(restored.toBytes())).to.deep.equal(Buffer.from(bytes));
    });
  });

  describe('recipientAddresses', () => {
    it('should return recipient addresses', () => {
      const transition = createTransition();
      const recipients = transition.recipientAddresses;
      expect(recipients).to.be.an('array');
      expect(recipients).to.have.lengthOf(1);
      expect(recipients[0].amount).to.equal(BigInt(90000));
    });

    it('should set recipient addresses', () => {
      const transition = createTransition();
      const newAddr = wasm.PlatformAddress.fromBytes(addr1Bytes);
      const newOutput = new wasm.PlatformAddressOutput(newAddr, BigInt(50000));

      transition.recipientAddresses = [newOutput];
      const recipients = transition.recipientAddresses;
      expect(recipients).to.have.lengthOf(1);
      expect(recipients[0].amount).to.equal(BigInt(50000));
    });
  });

  describe('senderId', () => {
    it('should return senderId', () => {
      const transition = createTransition();
      expect(transition.senderId.toBase58()).to.equal('11111111111111111111111111111111');
    });

    it('should set senderId with Identifier', () => {
      const transition = createTransition();
      const newSender = new wasm.Identifier('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');

      transition.senderId = newSender;
      expect(transition.senderId.toBase58()).to.equal('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
    });

    it('should set senderId with string', () => {
      const transition = createTransition();
      transition.senderId = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';
      expect(transition.senderId.toBase58()).to.equal('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
    });
  });

  describe('nonce', () => {
    it('should return nonce', () => {
      const transition = createTransition();
      expect(transition.nonce).to.equal(BigInt(1));
    });

    it('should set nonce', () => {
      const transition = createTransition();
      transition.nonce = BigInt(999);
      expect(transition.nonce).to.equal(BigInt(999));
    });
  });

  describe('signature', () => {
    it('should return empty signature by default', () => {
      const transition = createTransition();
      expect(transition.signature).to.deep.equal(Uint8Array.from([]));
    });

    it('should set signature', () => {
      const transition = createTransition();
      transition.signature = [1, 2, 3];
      expect(transition.signature).to.deep.equal(Uint8Array.from([1, 2, 3]));
    });
  });

  describe('signaturePublicKeyId', () => {
    it('should return default signaturePublicKeyId', () => {
      const transition = createTransition();
      expect(transition.signaturePublicKeyId).to.equal(0);
    });

    it('should set signaturePublicKeyId', () => {
      const transition = createTransition();
      transition.signaturePublicKeyId = 5;
      expect(transition.signaturePublicKeyId).to.equal(5);
    });
  });

  describe('userFeeIncrease', () => {
    it('should return default userFeeIncrease', () => {
      const transition = createTransition();
      expect(transition.userFeeIncrease).to.equal(0);
    });

    it('should set userFeeIncrease', () => {
      const transition = createTransition();
      transition.userFeeIncrease = 42;
      expect(transition.userFeeIncrease).to.equal(42);
    });
  });

  describe('toObject() / toJSON() / fromObject() / fromJSON()', () => {
    it('toObject() emits recipientAddresses as typed array of {address, amount}', () => {
      const transition = createTransition();
      const obj = transition.toObject();

      expect(obj.recipientAddresses).to.be.an('array').with.lengthOf(1);
      expect(obj.recipientAddresses[0].address).to.be.instanceOf(Uint8Array);
      expect(obj.recipientAddresses[0].address.length).to.equal(21);
      expect(obj.recipientAddresses[0].amount).to.equal(BigInt(90000));
    });

    it('toJSON() emits recipientAddresses with hex addresses and number/string amounts', () => {
      const transition = createTransition();
      const json = transition.toJSON();

      expect(json.recipientAddresses).to.be.an('array').with.lengthOf(1);
      expect(json.recipientAddresses[0].address).to.be.a('string').with.lengthOf(42);
      expect(json.recipientAddresses[0].amount).to.satisfy((v: unknown) => typeof v === 'number' || typeof v === 'string');
    });

    it('fromObject(toObject()) round-trips identically', () => {
      const transition = createTransition();
      const obj = transition.toObject();
      const restored = wasm.IdentityCreditTransferToAddresses.fromObject(obj);
      expect(restored.toObject()).to.deep.equal(obj);
    });

    it('fromJSON(toJSON()) round-trips identically', () => {
      const transition = createTransition();
      const json = transition.toJSON();
      const restored = wasm.IdentityCreditTransferToAddresses.fromJSON(json);
      expect(restored.toJSON()).to.deep.equal(json);
    });
  });

  describe('toStateTransition() / fromStateTransition()', () => {
    it('should convert to and from StateTransition', () => {
      const transition = createTransition();
      const st = transition.toStateTransition();
      expect(st).to.exist();

      const restored = wasm.IdentityCreditTransferToAddresses.fromStateTransition(st);
      expect(Buffer.from(restored.toBytes())).to.deep.equal(
        Buffer.from(transition.toBytes()),
      );
    });
  });
});
