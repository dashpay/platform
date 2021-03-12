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
            console.log(config.getName());

            const response = await tenderdashRpcClient.request('status', {})
              .catch((e) => {
                console.log(e);
              });

            console.log(response);

            if (response) {
              success = !response.error;
            }

            if (!success) {
              await wait(2000);
            }
          } while (!success);
        },
      },
    ]);
  }

  return waitForNodeToBeReadyTask;
}

module.exports = waitForNodeToBeReadyTaskFactory;
