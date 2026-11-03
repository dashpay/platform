/**
 * `documentCreateCost`: what creating a document of a type costs, computed by
 * Drive from the contract alone (no connection). The Drive test
 * `should_price_what_drive_charges` holds the storage to what Drive charges;
 * this checks the binding, its options and the shape JS receives.
 */
import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';

const ownerId = '11111111111111111111111111111111';

type Scenarios = { newValues: number; knownValues: number };
type Cost = {
  documentType: string;
  documentBytes: number;
  creditsPerByte: number;
  creditsPerDash: number;
  storage: { bytes: Scenarios; credits: Scenarios; primaryBytes: Scenarios; preallocatedBytes: Scenarios };
  indexes: Array<{ name: string; sharedWith: string[]; sharedBytes: Scenarios; ownBytes: Scenarios }>;
  processing: Array<{ code: string; credits: Scenarios; exact: boolean }>;
  processingCredits: Scenarios;
  contractCharges: Array<{ kind: string; charged?: { owner: number; moderators: number } }>;
  refund: { sameEpoch: Scenarios; afterOneYear: Scenarios };
  totalCredits: Scenarios;
  fields: Array<{
    path: string;
    kind: string;
    optional: boolean;
    present: boolean;
    length?: number;
    maxLength?: number;
  }>;
};

const schemas = {
  note: {
    type: 'object',
    documentsMutable: true,
    canBeDeleted: true,
    properties: {
      tag: { type: 'string', maxLength: 20, position: 0 },
      text: { type: 'string', maxLength: 60, position: 1 },
      mood: { type: 'string', maxLength: 10, position: 2 },
    },
    indices: [
      { name: 'byTag', properties: [{ tag: 'asc' }] },
      { name: 'byTagText', properties: [{ tag: 'asc' }, { text: 'asc' }] },
    ],
    required: ['tag', 'text'],
    additionalProperties: false,
    actionFees: { pricing: 'fixed', create: { owner: 1000, moderators: 500 } },
  },
  /** Kept for a day. */
  memo: {
    type: 'object',
    ttl: 86400,
    documentsMutable: true,
    properties: {
      text: { type: 'string', maxLength: 60, position: 0 },
    },
    indices: [{ name: 'byText', properties: [{ text: 'asc' }] }],
    required: ['$createdAt', 'text'],
    additionalProperties: false,
  },
};

describe('documentCreateCost()', () => {
  let contract: sdk.DataContract;

  before(async () => {
    await init();
    contract = new sdk.DataContract({
      ownerId,
      identityNonce: BigInt(1),
      schemas,
      fullValidation: true,
      platformVersion: new sdk.PlatformVersion(14),
    });
  });

  function cost(options?: unknown, documentType = 'note'): Cost {
    return sdk.documentCreateCost(
      contract,
      documentType,
      options as sdk.DocumentCreateCostOptions,
      new sdk.PlatformVersion(14),
    ) as unknown as Cost;
  }

  it('should price a document of middle sizes in credits, as numbers', () => {
    const result = cost();
    expect(result.documentType).to.equal('note');
    expect(result.creditsPerDash).to.equal(100_000_000_000);
    expect(result.storage.bytes.newValues).to.be.a('number');
    expect(result.storage.credits.newValues).to.equal(
      result.storage.bytes.newValues * result.creditsPerByte,
    );
    // A later document with the same tag adds less.
    expect(result.storage.bytes.knownValues).to.be.lessThan(result.storage.bytes.newValues);
    const tag = result.fields.find((f) => f.path === 'tag');
    expect(tag).to.include({ kind: 'string', optional: false, present: true, length: 10, maxLength: 20 });
  });

  it('should split the storage into what indexes share and what each adds', () => {
    const byTag = cost().indexes.find((i) => i.name === 'byTag');
    expect(byTag?.sharedWith).to.deep.equal(['byTagText']);
    expect(byTag?.sharedBytes.newValues).to.be.greaterThan(0);
    expect(byTag?.ownBytes.newValues).to.be.greaterThan(0);
  });

  it('should grow with a longer value and shrink without an optional one', () => {
    const middle = cost().storage.bytes.newValues;
    const longer = cost({ fields: { text: { length: 60 } } }).storage.bytes.newValues;
    const without = cost({ fields: { mood: { present: false } } });
    expect(longer).to.be.greaterThan(middle);
    expect(without.storage.bytes.newValues).to.be.lessThan(middle);
    expect(without.fields.find((f) => f.path === 'mood')).to.include({ present: false });
  });

  it('should add the action fee and itemize the processing', () => {
    const result = cost();
    expect(result.contractCharges[0]).to.deep.include({
      kind: 'actionFee',
      charged: { owner: 1000, moderators: 500 },
    });
    expect(result.totalCredits.newValues).to.equal(
      result.storage.credits.newValues + result.processingCredits.newValues + 1500,
    );
    const signature = result.processing.find((p) => p.code === 'signature');
    expect(signature).to.deep.include({ exact: true, credits: { newValues: 15000, knownValues: 15000 } });
    const bls = cost({ signatureKeyType: 'BLS12_381' }).processing.find((p) => p.code === 'signature');
    expect(bls?.credits.newValues).to.equal(300000);
    expect(result.refund.sameEpoch.newValues).to.be.lessThan(result.storage.credits.newValues);
  });

  it('should price a document with a ttl by its lifetime, with no refund', () => {
    const memo = cost(undefined, 'memo');
    expect(memo.creditsPerByte).to.be.lessThan(cost().creditsPerByte);
    expect(memo.refund.sameEpoch.newValues).to.equal(0);
    expect(memo.processing.map((p) => p.code)).to.include('ttlCleanup');
  });

  it('should refuse an unknown field or option', () => {
    expect(() => cost({ fields: { nope: { length: 1 } } })).to.throw(/nope/);
    expect(() => cost({ existingDocs: 5 })).to.throw(/existingDocs/);
    expect(() => cost({ fields: { text: { size: 5 } } })).to.throw(/size/);
  });
});
