import 'mocha';
import { expect } from 'chai';
import { createAccountReference } from "./createAccountReference";
import { HDPublicKey, PrivateKey } from "@dashevo/dashcore-lib";

const senderPrivateKey1 = new PrivateKey('2fc4145c8b7a871c42e32733a83c36f9b0d0eb646f40e53cb9ae0f48669ab0d7');
const extendedPublicKey1 = new HDPublicKey('tpubDMDatc2kPVD8R6hW2gQnmNDJ4xANWYueibmhPJoRnCjnagrTrRdFCDrzwD4bWaacsL4mms8dRyvWNLtzFYCuguTcXQRbiza1FnnFKT21GC6');

const extendedPublicKey2 = new HDPublicKey('tpubDLBwGAKsAffoGpUhHcceos8kkJh9ZTRSkjEm25ZbCrQMK9d68hi2yDjkWfCbdgaugmRrTxWQDRUd8Mb6SiXKCeosaSKT5piAhr7emPABFwJ');

describe('DashPayPlugin - createAccountReference', () => {
  it('create an account reference', function () {
    expect(createAccountReference(senderPrivateKey1.toBuffer(), extendedPublicKey1.toBuffer())).to.deep.equal(70557813);
    expect(createAccountReference(senderPrivateKey1.toBuffer(), extendedPublicKey2.toBuffer())).to.deep.equal(118374606);
  });

});
