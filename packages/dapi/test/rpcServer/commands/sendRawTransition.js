const chai = require('chai');
const Schema = require('@dashevo/dash-schema/dash-schema-lib');
const { PrivateKey, Transaction } = require('@dashevo/dashcore-lib');
const { createStateTransition, doubleSha256 } = require('../../../lib/rpcServer/commands/sendRawTransition');

const { expect } = chai;

describe('sendRawTransition', () => {
  describe('#doubleSha256', () => {
    it('should return a doubleSha256 hex digest string given a String', () => {
      const expected = '464472b56079ded3d359b17935624bdb8487b6a64856090725277ddb5fb5576a';
      const actual = doubleSha256('data');
      expect(actual).to.equal(expected);
    });
    it('should return a doubleSha256 hex digest string given a Buffer', () => {
      const dataBuffer = Buffer.from('data');
      const expected = '464472b56079ded3d359b17935624bdb8487b6a64856090725277ddb5fb5576a';
      const actual = doubleSha256(dataBuffer);
      expect(actual).to.equal(expected);
    });
  });
  describe('#createStateTransition', () => {
    it('should return a stateTransition', () => {
      let rawTransitionHeader = '03000c00000000000000ac01003c0a168a4d512742516a80a94293ad86ab2cb547415e8b96719a89f91048dfd03c0a168a4d512742516a80a94293ad86ab2cb547415e8b96719a89f91048dfd0e803000000000000f10b1c3217f0982a76623ae2639305f6ad788afbfedc89b584bbcd10f8a912c3411f55df779a07a9e395413bab34a97d003bf185e7f5d6116c5a9fd8a7fee582c7f076aa48f44902740e2784cd18adf4478374f25804d082f2ae8b886425742af1d4';
      const transitionDataPacket = {
        stpacket: {
          pver: 1,
          dapid: 'af462ee93b79b6991ebdc569f84c681c77525ad679d1c8b01087dbbbfbb3658d',
          dapcontract: {
            pver: 1,
            idx: 0,
            dapschema: {},
            dapver: 1,
            dapname: 'dapname',
            meta: {
              id: 'af462ee93b79b6991ebdc569f84c681c77525ad679d1c8b01087dbbbfbb3658d',
            },
          },
        },
      };

      const rawTransitionDataPacket = Schema.serialize.encode(transitionDataPacket).toString('hex');
      const rawTransitionDataPacketHexBuffer = Buffer.from(rawTransitionDataPacket, 'hex');
      const packetHash = doubleSha256(rawTransitionDataPacketHexBuffer);
      const headerTransaction = new Transaction(rawTransitionHeader);

      headerTransaction.extraPayload.setHashSTPacket(packetHash);
      const privateKey = new PrivateKey();
      headerTransaction.extraPayload.sign(privateKey);

      rawTransitionHeader = headerTransaction.serialize();

      const expected = {
        headerTransaction,
        packet: transitionDataPacket,
      };
      // eslint-disable-next-line no-underscore-dangle
      expected.headerTransaction._inputAmount = undefined;
      // eslint-disable-next-line no-underscore-dangle
      expected.headerTransaction._outputAmount = undefined;

      const actual = createStateTransition({ rawTransitionHeader, rawTransitionDataPacket });

      expect(actual).to.be.deep.equal(expected);
    });
  });
});
