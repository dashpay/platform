/**
 * `documentTypeLayout`: the GroveDB layout of a document type, computed by
 * Drive from the contract alone (no connection). The Drive test
 * `should_lay_out_what_drive_writes` holds the layout to what Drive writes;
 * this checks the binding and the shape JS receives.
 */
import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';

const ownerId = '11111111111111111111111111111111';

type LayoutNode = {
  key: { kind: string; hex?: string; label?: string; property?: string; components?: string[] };
  role: string;
  structureNode: string;
  element: string;
  wrapper?: string;
  rankedAxes: string[];
  indexes: string[];
  notes: Array<{ code: string; text: string }>;
  alternative?: { when: string; node: LayoutNode };
  children: LayoutNode[];
};

const schemas = {
  /** Plain, countable, unique, compound and ranked indexes over one type. */
  review: {
    type: 'object',
    properties: {
      shop: { type: 'string', maxLength: 32, position: 0 },
      rating: { type: 'integer', minimum: 1, maximum: 5, position: 1 },
      code: { type: 'string', maxLength: 16, position: 2 },
    },
    indices: [
      { name: 'byShop', properties: [{ shop: 'asc' }], countable: 'countable' },
      { name: 'byShopRating', properties: [{ shop: 'asc' }, { rating: 'asc' }] },
      { name: 'byCode', properties: [{ code: 'asc' }], unique: true },
      { name: 'topRated', properties: [{ rating: 'asc' }], countable: 'countable', rangeCountable: true, rankedCountable: true },
      {
        name: 'recent',
        properties: [{ $createdAt: 'asc' }],
        timeRange: { on: '$createdAt', range: 86400, step: 3600 },
        countable: 'countable',
      },
    ],
    required: ['shop', 'rating', '$createdAt'],
    additionalProperties: false,
  },
  /** An indexOnly type keyed by its owner. */
  like: {
    type: 'object',
    indexOnly: true,
    documentsMutable: false,
    canBeDeleted: true,
    properties: {
      shop: { type: 'string', maxLength: 32, position: 0 },
    },
    indices: [
      { name: 'byShop', properties: [{ shop: 'asc' }], terminal: '$ownerId' },
    ],
    required: ['shop'],
    additionalProperties: false,
  },
};

function child(node: LayoutNode, label: string): LayoutNode {
  const found = node.children.find((c) => c.key.label === label);
  if (!found) {
    throw new Error(`no child ${label} under ${node.key.label ?? node.key.kind}`);
  }
  return found;
}

describe('documentTypeLayout()', () => {
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

  function layout(name: string) {
    return sdk.documentTypeLayout(contract, name, new sdk.PlatformVersion(14)) as unknown as {
      documentType: string;
      root: LayoutNode;
    };
  }

  it('should describe the document type tree, the documents and one tree per first index property', () => {
    const { documentType, root } = layout('review');

    expect(documentType).to.equal('review');
    expect(root).to.include({ role: 'documentType', element: 'Tree' });
    expect(root.structureNode).to.equal('contracts.contract.documents.document_type');
    expect(root.indexes).to.have.members(['byShop', 'byShopRating', 'byCode', 'topRated', 'recent']);

    const primary = child(root, 'PrimaryKey');
    expect(primary.role).to.equal('primaryKey');
    expect(primary.children[0]).to.include({ role: 'document', element: 'Item' });
    expect(primary.children[0].key.kind).to.equal('documentId');

    expect(root.children.map((c) => c.key.label)).to.have.members([
      'PrimaryKey', 'shop', 'code', 'rating', '$createdAt#86400#3600',
    ]);
  });

  it('should key a time window level by its grid and give the grid as numbers', () => {
    const windows = child(layout('review').root, '$createdAt#86400#3600').children[0];

    expect(windows.key).to.deep.equal({
      kind: 'timeRangeBucket', property: '$createdAt', rangeSeconds: 86400, stepSeconds: 3600, phaseSeconds: 0,
    });
    expect(windows.notes.map((n) => n.code)).to.include('timeRangeOverlap');
  });

  it('should count at a countable terminal and share the prefix of a compound index', () => {
    const shop = child(layout('review').root, 'shop');
    expect(shop.indexes).to.have.members(['byShop', 'byShopRating']);

    const value = shop.children[0];
    expect(value).to.include({ role: 'indexValue' });
    expect(value.key).to.deep.equal({ kind: 'propertyValue', property: 'shop' });

    const terminal = child(value, 'Members');
    expect(terminal).to.include({ role: 'terminal', element: 'CountTree' });
    expect(terminal.indexes).to.deep.equal(['byShop']);
    expect(terminal.children[0]).to.include({ role: 'member', element: 'Reference' });

    // The compound index continues under the counted value tree, so its
    // property tree contributes nothing to that count.
    const rating = child(value, 'rating');
    expect(rating).to.include({ role: 'nextIndexProperty', wrapper: 'NonCounted' });
    expect(rating.indexes).to.deep.equal(['byShopRating']);
  });

  it('should write a unique index reference straight at [0], or a tree when a value is null', () => {
    const value = child(layout('review').root, 'code').children[0];
    const terminal = value.children[0];

    expect(terminal).to.include({ role: 'terminal', element: 'Reference' });
    expect(terminal.alternative?.when).to.match(/null/);
    expect(terminal.alternative?.node.children[0].key.kind).to.equal('documentId');
  });

  it('should rank a ranked index in an indexed tree', () => {
    const rating = child(layout('review').root, 'rating');

    expect(rating.element).to.equal('ProvableCountIndexedTree');
    expect(rating.rankedAxes).to.deep.equal(['count']);
  });

  it('should key an indexOnly entry by its terminal and store no documents', () => {
    const { root } = layout('like');

    expect(root.notes.map((n) => n.code)).to.include('indexOnly');
    expect(root.children.map((c) => c.key.label)).to.deep.equal(['shop']);

    const member = child(child(root, 'shop').children[0], 'Members').children[0];
    expect(member).to.include({ role: 'member', element: 'Item' });
    expect(member.key).to.deep.equal({ kind: 'memberKey', components: ['$ownerId'] });
  });

  it('should refuse a document type the contract lacks', () => {
    expect(() => sdk.documentTypeLayout(contract, 'missing', new sdk.PlatformVersion(14))).to.throw(/missing/);
  });
});
