const { Listr } = require('listr2');
const isEqual = require('lodash.isequal');

const wait = require('../../../../util/wait');

const { LLMQ_TYPE_TEST, NETWORK_LOCAL } = require('../../../../constants');

const waitForQuorumPhase = require('../../../../core/quorum/waitForQuorumPhase');
const waitForNodesToHaveTheSameHeight = require('../../../../core/waitForNodesToHaveTheSameHeight');
const waitForQuorumConnections = require('../../../../core/quorum/waitForQuorumConnections');
const waitForMasternodeProbes = require('../../../../core/quorum/waitForMasternodeProbes');
const waitForQuorumCommitments = require('../../../../core/quorum/waitForQuorumCommitements');

/**
 * @param {generateBlocks} generateBlocks
 * @return {enableCoreQuorumsTask}
 */
function enableCoreQuorumsTaskFactory(generateBlocks) {
  /**
   * @typedef {enableCoreQuorumsTask}
   * @return {Listr}
   */
  function enableCoreQuorumsTask() {
    const WAIT_FOR_NODES_TIMEOUT = 60 * 5 * 1000;

    return new Listr([
      {
        task: (ctx) => {
          // Those are default values for the quorum size 3 with all nodes
          // behaving correctly with "llmq_test" quorum
          ctx.expectedMembers = 3;
          ctx.expectedCommitments = 3;
          ctx.expectedConnections = 2;

          ctx.expectedContributions = 3;
          ctx.expectedJustifications = 0;
          ctx.expectedComplaints = 0;
        },
      },
      {
        title: 'Start DKG session',
        task: async (ctx) => {
          const { result: initialQuorumList } = await ctx.seedRpcClient.quorum('list');

          ctx.initialQuorumList = initialQuorumList;

          const { result: bestBlockHeight } = await ctx.seedRpcClient.getBlockCount();

          // move forward to next DKG
          const blocksUntilNextDKG = 24 - (bestBlockHeight % 24);
          if (blocksUntilNextDKG !== 0) {
            await ctx.bumpMockTime();

            await generateBlocks(
              ctx.seedCoreService,
              blocksUntilNextDKG,
              NETWORK_LOCAL,
            );
          }

          await waitForNodesToHaveTheSameHeight(
            ctx.rpcClients,
            WAIT_FOR_NODES_TIMEOUT,
          );
        },
      },
      {
        title: 'Waiting for phase 1 (init)',
        task: async (ctx) => {
          const { result: quorumHash } = await ctx.seedRpcClient.getBestBlockHash();

          ctx.quorumHash = quorumHash;

          await waitForQuorumPhase(
            ctx.rpcClients,
            ctx.quorumHash,
            1,
            ctx.expectedMembers,
          );

          await waitForQuorumConnections(
            ctx.rpcClients,
            ctx.expectedConnections,
            ctx.bumpMockTime,
          );

          const { result: sporks } = await ctx.seedRpcClient.spork('show');
          const isSpork21Active = sporks.SPORK_21_QUORUM_ALL_CONNECTED === 0;

          if (isSpork21Active) {
            await waitForMasternodeProbes(
              ctx.rpcClients,
              ctx.bumpMockTime,
            );
          }

          await ctx.bumpMockTime();

          await generateBlocks(
            ctx.seedCoreService,
            2,
            NETWORK_LOCAL,
          );

          await waitForNodesToHaveTheSameHeight(
            ctx.rpcClients,
            WAIT_FOR_NODES_TIMEOUT,
          );
        },
      },
      {
        title: 'Waiting for phase 2 (contribute)',
        task: async (ctx) => {
          await waitForQuorumPhase(
            ctx.rpcClients,
            ctx.quorumHash,
            2,
            ctx.expectedMembers,
          );

          await ctx.bumpMockTime();

          await generateBlocks(
            ctx.seedCoreService,
            2,
            NETWORK_LOCAL,
          );

          await waitForNodesToHaveTheSameHeight(
            ctx.rpcClients,
            WAIT_FOR_NODES_TIMEOUT,
          );
        },
      },
      {
        title: 'Waiting for phase 3 (complain)',
        task: async (ctx) => {
          await waitForQuorumPhase(
            ctx.rpcClients,
            ctx.quorumHash,
            3,
            ctx.expectedMembers,
            'receivedComplaints',
            ctx.expectedComplaints,
          );

          await ctx.bumpMockTime();

          await generateBlocks(
            ctx.seedCoreService,
            2,
            NETWORK_LOCAL,
          );

          await waitForNodesToHaveTheSameHeight(
            ctx.rpcClients,
            WAIT_FOR_NODES_TIMEOUT,
          );
        },
      },
      {
        title: 'Waiting for phase 4 (justify)',
        task: async (ctx) => {
          await waitForQuorumPhase(
            ctx.rpcClients,
            ctx.quorumHash,
            4,
            ctx.expectedMembers,
            'receivedJustifications',
            ctx.expectedJustifications,
          );

          await ctx.bumpMockTime();

          await generateBlocks(
            ctx.seedCoreService,
            2,
            NETWORK_LOCAL,
          );

          await waitForNodesToHaveTheSameHeight(
            ctx.rpcClients,
            WAIT_FOR_NODES_TIMEOUT,
          );
        },
      },
      {
        title: 'Waiting for phase 5 (commit)',
        task: async (ctx) => {
          await waitForQuorumPhase(
            ctx.rpcClients,
            ctx.quorumHash,
            5,
            ctx.expectedMembers,
            'receivedPrematureCommitments',
            ctx.expectedCommitments,
          );

          await ctx.bumpMockTime();

          await generateBlocks(
            ctx.seedCoreService,
            2,
            NETWORK_LOCAL,
          );

          await waitForNodesToHaveTheSameHeight(
            ctx.rpcClients,
            WAIT_FOR_NODES_TIMEOUT,
          );
        },
      },
      {
        title: 'Waiting for phase 6 (mining)',
        task: async (ctx) => {
          await waitForQuorumPhase(
            ctx.rpcClients,
            ctx.quorumHash,
            6,
            ctx.expectedMembers,
          );
        },
      },
      {
        title: 'Waiting final commitment',
        task: (ctx) => waitForQuorumCommitments(
          ctx.rpcClients,
          ctx.quorumHash,
        ),
      },
      {
        title: 'Mining final commitment',
        task: async (ctx, task) => {
          await ctx.bumpMockTime();

          await generateBlocks(
            ctx.seedCoreService,
            1,
            NETWORK_LOCAL,
          );

          let { result: newQuorumList } = await ctx.seedRpcClient.quorum('list');

          while (isEqual(ctx.initialQuorumList, newQuorumList)) {
            await wait(300);

            await ctx.bumpMockTime();

            await generateBlocks(
              ctx.seedCoreService,
              1,
              NETWORK_LOCAL,
            );

            await waitForNodesToHaveTheSameHeight(
              ctx.rpcClients,
              WAIT_FOR_NODES_TIMEOUT,
            );

            ({ result: newQuorumList } = await ctx.seedRpcClient.quorum('list'));
          }

          const { result: quorumList } = await ctx.seedRpcClient.quorum('list', 1);

          const newQuorumHash = quorumList[LLMQ_TYPE_TEST][0];

          const { result: quorumInfo } = await ctx.seedRpcClient.quorum('info', 100, newQuorumHash);

          // Mine 8 (SIGN_HEIGHT_OFFSET) more blocks to make sure
          // that the new quorum gets eligable for signing sessions
          await generateBlocks(
            ctx.seedCoreService,
            8,
            NETWORK_LOCAL,
          );

          await waitForNodesToHaveTheSameHeight(
            ctx.rpcClients,
            WAIT_FOR_NODES_TIMEOUT,
          );

          // eslint-disable-next-line no-param-reassign
          task.output = `New quorum mined: height: ${quorumInfo.height}, quorum hash: ${newQuorumHash}, mined in block: ${quorumInfo.minedBlock}`;
        },
      },
    ]);
  }

  return enableCoreQuorumsTask;
}

module.exports = enableCoreQuorumsTaskFactory;
