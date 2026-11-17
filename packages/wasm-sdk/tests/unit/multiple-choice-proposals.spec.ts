/**
 * Multiple choice proposals (protocol version 15). `buildMultipleChoiceProposalEdit` and
 * `getMultipleChoiceProposalVotePoll` need no network. The other calls parse every option
 * before anything is fetched, so a call whose only fault is one option fails on that option:
 * the options before it were accepted, and nothing reached the network.
 */
import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';

const DECISION_PROPOSALS_CONTRACT_ID = '3iNyoNwjCTkb7XS3RHyWUtLx7pgC6AFAEQNNDHzxqngF';
const ownerId = '5mjGWa9mruHnLBht3ntBi8CZ6sNk3hZZsQMgTvgQobjS';
const proposalId = '4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF';
const descriptionId = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';
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

describe('multiple choice proposals', () => {
  let client: sdk.WasmSdk;

  before(async () => {
    await init();
    client = await sdk.WasmSdkBuilder.testnet().build();
  });

  after(() => {
    client?.free();
  });

  function storedProposal(documentTypeName = 'multipleChoiceProposal'): sdk.Document {
    return new sdk.Document({
      properties: {
        title: 'Which block size?',
        descriptionId: new Uint8Array(32).fill(5),
        descriptionContractId: new Uint8Array(32).fill(6),
        descriptionDocumentType: 'post',
        url: 'https://forum.dash.org/t/block-size',
        open: true,
      },
      documentTypeName,
      dataContractId: DECISION_PROPOSALS_CONTRACT_ID,
      ownerId,
      revision: BigInt(1),
      entropy: new Uint8Array(32).fill(3),
    });
  }

  describe('buildMultipleChoiceProposalEdit()', () => {
    it('should give the proposal a new title and its next revision', () => {
      const proposal = storedProposal();
      const edited = client.buildMultipleChoiceProposalEdit(proposal, { title: 'Which block size, then?' });
      expect(edited.id.toBase58()).to.equal(proposal.id.toBase58());
      expect(edited.revision).to.equal(BigInt(2));
      const properties = edited.properties as Record<string, unknown>;
      expect(properties.title).to.equal('Which block size, then?');
      expect(properties.url).to.equal('https://forum.dash.org/t/block-size');
      expect(properties.descriptionDocumentType).to.equal('post');
      expect(properties.open).to.equal(true);
      expect(edited.documentTypeName).to.equal('multipleChoiceProposal');
    });

    it('should remove all three description properties when the edit sets the document to null', () => {
      const edited = client.buildMultipleChoiceProposalEdit(storedProposal(), { descriptionDocument: null });
      const properties = edited.properties as Record<string, unknown>;
      expect(properties).to.not.have.property('descriptionId');
      expect(properties).to.not.have.property('descriptionContractId');
      expect(properties).to.not.have.property('descriptionDocumentType');
      expect(properties.url).to.equal('https://forum.dash.org/t/block-size');
    });

    it('should point the proposal at another description document and remove the url', () => {
      const edited = client.buildMultipleChoiceProposalEdit(storedProposal(), {
        descriptionDocument: { id: proposalId, contractId: descriptionId, documentType: 'note' },
        url: null,
      });
      const properties = edited.properties as Record<string, unknown>;
      expect(properties.descriptionDocumentType).to.equal('note');
      expect(properties).to.not.have.property('url');
    });

    it('should refuse an edit that changes nothing', () => {
      const error = refusalOf(() => client.buildMultipleChoiceProposalEdit(storedProposal(), {}));
      expect(error.message).to.match(/changes nothing/);
    });

    it('should refuse a document that is not a multiple choice proposal', () => {
      const error = refusalOf(() => client.buildMultipleChoiceProposalEdit(
        storedProposal('proposal'),
        { title: 'Another question?' },
      ));
      expect(error.message).to.match(/is not a multiple choice proposal/);
    });
  });

  describe('getMultipleChoiceProposalVotePoll()', () => {
    it('should name the contest of the choice type at the proposal', () => {
      const votePoll = client.getMultipleChoiceProposalVotePoll(proposalId);
      const json = votePoll.toJSON() as Record<string, unknown>;
      expect(JSON.stringify(json)).to.contain('byProposal');
      expect(JSON.stringify(json)).to.contain(DECISION_PROPOSALS_CONTRACT_ID);
    });
  });

  describe('listings', () => {
    it('should accept Identifier instances as the owner and the cursor', async () => {
      const error = await rejectionOf(client.getMultipleChoiceProposalsByOwner({
        ownerId: new sdk.Identifier(ownerId),
        startAfter: new sdk.Identifier(ownerId),
        limit: 1.5,
      }));
      expect(error.message).to.match(/'limit' must be an integer/);
    });
  });

  describe('buildMultipleChoiceProposal()', () => {
    it('should refuse a description document that names no contract', async () => {
      const error = await rejectionOf(client.buildMultipleChoiceProposal({
        ownerId,
        title: 'Which block size?',
        descriptionDocument: { id: descriptionId, documentType: 'post' } as never,
        open: true,
      }));
      expect(error.message).to.match(/contractId/);
    });

    it('should refuse an open that is not a boolean', async () => {
      const error = await rejectionOf(client.buildMultipleChoiceProposal({
        ownerId,
        title: 'Which block size?',
        url: 'https://forum.dash.org/t/block-size',
        open: 'yes' as never,
      }));
      expect(error.message).to.match(/'open' must be a boolean/);
    });
  });

  describe('multipleChoiceProposalVote()', () => {
    it('should refuse an answer that is neither a choice, abstain nor none of the choices', async () => {
      const error = await rejectionOf(client.multipleChoiceProposalVote({
        masternodeProTxHash: proTxHashHex,
        proposalId,
        voteChoice: 'lock' as never,
      } as never));
      expect(error.message).to.match(/'abstain' or 'noneOfTheChoices', got 'lock'/);
    });

    [{ choiceId: descriptionId }, 'abstain', 'noneOfTheChoices'].forEach((voteChoice) => {
      it(`should accept ${JSON.stringify(voteChoice)} and read the voting key before the network`, async () => {
        const error = await rejectionOf(client.multipleChoiceProposalVote({
          masternodeProTxHash: proTxHashHex,
          proposalId,
          voteChoice,
        } as never));
        expect(error.message).to.match(/'votingKey' is required/);
      });
    });
  });
});
