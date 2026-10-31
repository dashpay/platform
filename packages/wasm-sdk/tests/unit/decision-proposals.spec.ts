/**
 * Decision proposals (protocol version 15). `buildDecisionProposalEdit` needs no network: it
 * gives a stored proposal new text and its next revision. The listings and the vote parse every
 * option before anything is fetched, so a call whose only fault is one option fails on that
 * option: the options before it were accepted, and nothing reached the network.
 */
import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';

const DECISION_PROPOSALS_CONTRACT_ID = '3iNyoNwjCTkb7XS3RHyWUtLx7pgC6AFAEQNNDHzxqngF';
const ownerId = '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS';
const proTxHashHex = 'a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2';

async function rejectionOf(promise: Promise<unknown>): Promise<Error> {
  try {
    await promise;
  } catch (e) {
    return e as Error;
  }
  throw new Error('expected the call to be refused');
}

function refusalOf(call: () => unknown): Error {
  try {
    call();
  } catch (e) {
    return e as Error;
  }
  throw new Error('expected the call to be refused');
}

describe('decision proposals', () => {
  let client: sdk.WasmSdk;

  before(async () => {
    await init();
    client = await sdk.WasmSdkBuilder.testnet().build();
  });

  after(() => {
    client?.free();
  });

  function storedProposal(dataContractId = DECISION_PROPOSALS_CONTRACT_ID): sdk.Document {
    return new sdk.Document({
      properties: {
        title: 'Raise the block size?',
        description: 'Yes doubles it; no keeps it.',
        url: 'https://forum.dash.org/t/block-size',
        passPercent: 60,
        minimumTurnoutPercent: 10,
        endDate: BigInt(1_700_000_000_000),
      },
      documentTypeName: 'proposal',
      dataContractId,
      ownerId,
      revision: BigInt(1),
      entropy: new Uint8Array(32).fill(3),
    });
  }

  describe('buildDecisionProposalEdit()', () => {
    it('should give the proposal new text and its next revision', () => {
      const proposal = storedProposal();
      const edited = client.buildDecisionProposalEdit(proposal, { title: 'Raise the block size to 4 MB?' });
      expect(edited.id.toBase58()).to.equal(proposal.id.toBase58());
      expect(edited.revision).to.equal(BigInt(2));
      const properties = edited.properties as Record<string, unknown>;
      expect(properties.title).to.equal('Raise the block size to 4 MB?');
      expect(properties.description).to.equal('Yes doubles it; no keeps it.');
      expect(properties.url).to.equal('https://forum.dash.org/t/block-size');
      expect(edited.documentTypeName).to.equal('proposal');
      expect(edited.dataContractId.toBase58()).to.equal(DECISION_PROPOSALS_CONTRACT_ID);
    });

    it('should remove the url when the edit sets it to null', () => {
      const edited = client.buildDecisionProposalEdit(storedProposal(), { description: 'Yes doubles it.', url: null });
      const properties = edited.properties as Record<string, unknown>;
      expect(properties.description).to.equal('Yes doubles it.');
      expect(properties).to.not.have.property('url');
    });

    it('should refuse an edit that changes nothing', () => {
      const error = refusalOf(() => client.buildDecisionProposalEdit(storedProposal(), {}));
      expect(error.message).to.match(/changes nothing/);
    });

    it('should refuse a document that is not a decision proposal', () => {
      const error = refusalOf(() => client.buildDecisionProposalEdit(
        storedProposal('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec'),
        { title: 'Another question?' },
      ));
      expect(error.message).to.match(/is not a decision proposal/);
    });
  });

  describe('listings', () => {
    it('should accept Identifier instances as the owner and the cursor', async () => {
      const error = await rejectionOf(client.getDecisionProposalsByOwner({
        ownerId: new sdk.Identifier(ownerId),
        startAfter: new sdk.Identifier(ownerId),
        limit: 1.5,
      }));
      expect(error.message).to.match(/'limit' must be an integer/);
    });

    it('should refuse an end date bound that is not a number', async () => {
      const error = await rejectionOf(client.getDecisionProposalsByEndDate({
        endDateFrom: 'soon' as never,
      }));
      expect(error.message).to.match(/endDateFrom/);
    });
  });

  describe('decisionProposalVote()', () => {
    it('should refuse an answer other than yes, no or abstain', async () => {
      const error = await rejectionOf(client.decisionProposalVote({
        masternodeProTxHash: proTxHashHex,
        proposalId: ownerId,
        voteChoice: 'maybe' as never,
      } as never));
      expect(error.message).to.match(/'yes', 'no' or 'abstain', got 'maybe'/);
    });
  });
});
