const { Listr } = require('listr2');
const wait = require('../../../util/wait');

/**
 *
 * @param {createTenderdashRpcClient} createTenderdashRpcClient
 * @return {waitForNodeToBeReadyTask}
 */
function waitForNodeToBeReadyTaskFactory(
  createTenderdashRpcClient,
) {
  /**
   * @typedef waitForNodeToBeReadyTask
   * @param {Config} config
   * @return {Promise<void>}
   */
  async function waitForNodeToBeReadyTask(config) {
    return new Listr([
      {
        task: async () => {
          const port = config.get('platform.drive.tenderdash.rpc.port');

          const tenderdashRpcClient = createTenderdashRpcClient({ port });

          let success = false;
          do {
            const response = await tenderdashRpcClient.request('status', {}).catch(() => {});

            if (response) {
              success = !response.result.sync_info.catching_up;
            }

            if (!success) {
              await wait(500);
            }
          } while (!success);
        },
      },
    ]);
  }

  return waitForNodeToBeReadyTask;
}

module.exports = waitForNodeToBeReadyTaskFactory;
