import { Listr } from 'listr2';
import { LLMQ_TYPE_1, NETWORK_LOCAL } from '../../../../constants.js';
import wait from '../../../../util/wait.js';
/**
 * @param {generateBlocks} generateBlocks
 * @return {enableSingleCoreQuorumTask}
 */
export default function enableSingleCoreQuorumTaskFactory(generateBlocks) {
  /**
   * @typedef {enableSingleCoreQuorumTask}
   * @return {Listr}
   */
  function enableSingleCoreQuorumTask() {
    return new Listr([
      {
        title: 'Wait for quorum',
        task: async (ctx) => {
          const seedCoreService = ctx.coreServices
            .filter((coreService) => coreService.getConfig().getName() === 'local_seed')[0];

          const seedRpcClient = seedCoreService.getRpcClient();

          let llmq1 = [];
          do {
            ({ result: { [LLMQ_TYPE_1]: llmq1 } } = await seedRpcClient.quorum('list'));

            await generateBlocks(
              seedCoreService,
              2,
              NETWORK_LOCAL,
            );

            await wait(300);
          } while (llmq1.length === 0);
        },
      },
    ]);
  }

  return enableSingleCoreQuorumTask;
}
