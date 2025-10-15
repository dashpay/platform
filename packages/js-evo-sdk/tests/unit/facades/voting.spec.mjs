import init, * as wasmSDKPackage from '@dashevo/wasm-sdk';
import { EvoSDK } from '../../../dist/sdk.js';

describe('VotingFacade', () => {
  let wasmSdk;
  let client;

  beforeEach(async function setup() {
    await init();
    const builder = wasmSDKPackage.WasmSdkBuilder.testnetTrusted();
    wasmSdk = builder.build();
    client = EvoSDK.fromWasm(wasmSdk);

    this.sinon.stub(wasmSdk, 'getContestedResourceVoteState').resolves('ok');
    this.sinon.stub(wasmSdk, 'getContestedResourceVoteStateWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getContestedResourceIdentityVotes').resolves('ok');
    this.sinon.stub(wasmSdk, 'getContestedResourceIdentityVotesWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getVotePollsByEndDate').resolves('ok');
    this.sinon.stub(wasmSdk, 'getVotePollsByEndDateWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'masternodeVote').resolves('ok');
  });

  it('contestedResourceVoteState and related queries forward correctly', async () => {
    await client.voting.contestedResourceVoteState({
      contractId: 'c', documentTypeName: 'dt', indexName: 'i', indexValues: ['v1'], resultType: 'rt',
    });
    await client.voting.contestedResourceVoteStateWithProof({
      contractId: 'c', documentTypeName: 'dt', indexName: 'i', indexValues: ['v1'], resultType: 'rt', allowIncludeLockedAndAbstainingVoteTally: true, startAtIdentifierInfo: 's', count: 2, orderAscending: false,
    });
    await client.voting.contestedResourceIdentityVotes('id', { limit: 3, startAtVotePollIdInfo: 's', orderAscending: true });
    await client.voting.contestedResourceIdentityVotesWithProof('id', { limit: 4, offset: 1, orderAscending: false });
    await client.voting.votePollsByEndDate({
      startTimeMs: 1000,
      startTimeIncluded: true,
      endTimeMs: 2000,
      endTimeIncluded: false,
      limit: 2,
      offset: 1,
      orderAscending: true,
    });
    await client.voting.votePollsByEndDateWithProof({
      startTimeMs: 10,
      endTimeMs: 20,
      limit: 1,
      offset: 0,
      orderAscending: false,
    });
    expect(wasmSdk.getContestedResourceVoteState).to.be.calledOnce();
    expect(wasmSdk.getContestedResourceVoteStateWithProofInfo).to.be.calledOnce();
    expect(wasmSdk.getContestedResourceIdentityVotes).to.be.calledOnce();
    expect(wasmSdk.getContestedResourceIdentityVotesWithProofInfo).to.be.calledOnce();
    expect(wasmSdk.getVotePollsByEndDate).to.be.calledOnce();
    expect(wasmSdk.getVotePollsByEndDateWithProofInfo).to.be.calledOnce();
  });

  it('masternodeVote() stringifies array indexValues and forwards', async () => {
    await client.voting.masternodeVote({
      masternodeProTxHash: 'h', contractId: 'c', documentTypeName: 'dt', indexName: 'i', indexValues: ['x', 'y'], voteChoice: 'yes', votingKeyWif: 'w',
    });
    const { args } = wasmSdk.masternodeVote.firstCall;
    expect(args).to.deep.equal(['h', 'c', 'dt', 'i', JSON.stringify(['x', 'y']), 'yes', 'w']);
  });
});
