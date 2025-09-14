import { EvoSDK } from '../../../dist/evo-sdk.module.js';
import sinon from 'sinon';

const isBrowser = typeof window !== 'undefined';

describe('VotingFacade', () => {
  if (!isBrowser) {
    it('skips in Node environment (browser-only)', function () { this.skip(); });
    return;
  }

  let wasmStubModule;
  before(async () => { wasmStubModule = await import('@dashevo/wasm-sdk'); });
  beforeEach(() => { wasmStubModule.__clearCalls(); });

  it('contestedResourceVoteState and related queries forward correctly', async () => {
    const raw = {};
    const sdk = EvoSDK.fromWasm(raw);
    await sdk.voting.contestedResourceVoteState({ contractId: 'c', documentTypeName: 'dt', indexName: 'i', indexValues: ['v1'], resultType: 'rt' });
    await sdk.voting.contestedResourceVoteStateWithProof({ contractId: 'c', documentTypeName: 'dt', indexName: 'i', indexValues: ['v1'], resultType: 'rt', allowIncludeLockedAndAbstainingVoteTally: true, startAtIdentifierInfo: 's', count: 2, orderAscending: false });
    await sdk.voting.contestedResourceIdentityVotes('id', { limit: 3, startAtVotePollIdInfo: 's', orderAscending: true });
    await sdk.voting.contestedResourceIdentityVotesWithProof('id', { limit: 4, offset: 1, orderAscending: false });
    await sdk.voting.votePollsByEndDate({ startTimeInfo: 'a', endTimeInfo: 'b', limit: 2, orderAscending: true });
    await sdk.voting.votePollsByEndDateWithProof({ startTimeMs: 10, endTimeMs: 20, limit: 1, offset: 0, orderAscending: false });
    const names = wasmStubModule.__getCalls().map(c => c.called);
    expect(names).to.include.members([
      'get_contested_resource_vote_state',
      'get_contested_resource_vote_state_with_proof_info',
      'get_contested_resource_identity_votes',
      'get_contested_resource_identity_votes_with_proof_info',
      'get_vote_polls_by_end_date',
      'get_vote_polls_by_end_date_with_proof_info',
    ]);
  });

  it('masternodeVote() stringifies array indexValues and forwards', async () => {
    const wasm = { masternodeVote: sinon.stub().resolves('ok') };
    const sdk = EvoSDK.fromWasm(wasm);
    await sdk.voting.masternodeVote({ masternodeProTxHash: 'h', contractId: 'c', documentTypeName: 'dt', indexName: 'i', indexValues: ['x', 'y'], voteChoice: 'yes', votingKeyWif: 'w' });
    const args = wasm.masternodeVote.firstCall.args;
    expect(args).to.deep.equal(['h', 'c', 'dt', 'i', JSON.stringify(['x', 'y']), 'yes', 'w']);
  });
});

